use pipewire as pw;
use pipewire::stream::{Stream, StreamFlags};
use pipewire::spa;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub async fn capture_frame(node_id: u32, width: u32, height: u32) -> Result<CapturedFrame, String> {
    pw::init();

    let mainloop = pw::main_loop::MainLoop::new(None).map_err(|e| e.to_string())?;
    let context = pw::context::Context::new(&mainloop).map_err(|e| e.to_string())?;
    let core = context.connect(None).map_err(|e| e.to_string())?;

    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    let mainloop_weak = mainloop.downgrade();

    let mut props = pw::properties::Properties::new();
    props.insert(*pw::keys::MEDIA_TYPE, "Video");
    props.insert(*pw::keys::MEDIA_CATEGORY, "Capture");
    props.insert(*pw::keys::MEDIA_ROLE, "Screen");

    let stream = Stream::new(&core, "theoshot-capture", props).map_err(|e| e.to_string())?;

    let _listener = stream
        .add_local_listener::<()>()
        .param_changed(move |stream, _user_data, id, param_pod| {
            if id != spa::sys::SPA_PARAM_EnumFormat && id != spa::sys::SPA_PARAM_Format {
                return;
            }
            if let Some(pod) = param_pod {
                let _ = stream.update_params(&mut [pod]);
            }
        })
        .process(move |stream, _user_data| {
            if let Some(mut buffer) = stream.dequeue_buffer() {
                let datas = buffer.datas_mut();
                if let Some(data) = datas.get_mut(0) {
                    let chunk = data.chunk();
                    let size = chunk.size() as usize;
                    let offset = chunk.offset() as usize;

                    if let Some(raw_data) = data.data() {
                        if !raw_data.is_empty() {
                            let mut res = result_clone.lock().unwrap();
                            if res.is_none() {
                                *res = Some(CapturedFrame {
                                    width,
                                    height,
                                    data: raw_data[offset..(offset + size)].to_vec(),
                                });
                                if let Some(ml) = mainloop_weak.upgrade() {
                                    ml.quit();
                                }
                            }
                        }
                    }
                }
            }
        })
        .register()
        .map_err(|e| e.to_string())?;

    let flags = StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS;

    // Tentamos o connect sem params. Se o compositor não mandar frames, 
    // ele enviará um EnumFormat que nós aceitaremos no callback acima.
    stream.connect(
        spa::utils::Direction::Input,
        Some(node_id),
        flags,
        &mut [],
    ).map_err(|e| e.to_string())?;

    let ml_timeout = mainloop.downgrade();
    let _timer = mainloop.loop_().add_timer(move |_| {
        if let Some(ml) = ml_timeout.upgrade() {
            ml.quit();
        }
    });
    _timer.update_timer(Some(Duration::from_secs(5)), None);

    mainloop.run();

    let final_data = result.lock().unwrap().take();
    final_data.ok_or_else(|| "Nenhum frame recebido.".to_string())
}
