include!(concat!(env!("OUT_DIR"), "/generated.rs"));

mod state;

use std::{cell, collections, future, rc};

pub trait Harness {
    type Config: Copy;

    fn new<'b>(
        config: Self::Config,
        device: &wgpu::Device,
        assets: collections::HashMap<&'b str, &'b [u8]>,
    ) -> impl future::Future<Output = anyhow::Result<Self>> where Self: Sized;

    fn update(&mut self, queue: &wgpu::Queue);

    fn submit_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface: &wgpu::TextureView,
    ) -> anyhow::Result<()>;

    fn handle_event(&mut self, event: winit::event::DeviceEvent) -> bool;
}

pub struct App<'a, T, H: Harness<Config = T>> {
    state: state::State<'a>,
    inner: H,
    event_loop: winit::event_loop::EventLoop<()>,
}

impl<'a, T, H: Harness<Config = T>> App<'a, T, H> {
    pub async fn new(
        config: T,
    ) -> Result<Self, String> where Self: Sized {
        #[cfg(target_arch = "wasm32")] {
            console_error_panic_hook::set_once();
            wasm_logger::init(wasm_logger::Config::default());
        }
        
        #[cfg(not(target_arch = "wasm32"))] {
            simple_logger::SimpleLogger::new()
                .with_level(log::LevelFilter::Info)
                .init()
                .unwrap();
        }

        async fn new_inner<'a, T, H: Harness<Config = T>>(
            config: T
        ) -> anyhow::Result<App<'a, T, H>> {
            let mut assets = collections::HashMap::new();
            for (tag, asset) in generate().into_iter() {
                assets.insert(tag, asset.data);
            }
            
            let event_loop = winit::event_loop::EventLoop::new()?;
    
            let state = {
                state::State::new(&event_loop).await
            }?;
    
            let inner = {
                H::new(config, &state.device, assets).await
            }?;
    
            Ok(App { state, inner, event_loop })
        }

        new_inner(config).await.map_err(|e| e.to_string())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update_canvas(
        w: wasm_bindgen::JsValue, h: wasm_bindgen::JsValue,
    ) -> Result<(), String> {
        unsafe fn update_canvas_inner(
            w: wasm_bindgen::JsValue, h: wasm_bindgen::JsValue,
        ) -> anyhow::Result<winit::dpi::PhysicalSize<u32>> {
            let width: u32 = w.as_string()
                .ok_or(state::err::WebError::new("parse canvas width"))?
                .parse()?;
        
            let height: u32 = h.as_string()
                .ok_or(state::err::WebError::new("parse canvas height"))?
                .parse()?;

            Ok(winit::dpi::PhysicalSize { width, height })
        }
    
        unsafe {
            let size = update_canvas_inner(w, h)
                .map_err(|e| e.to_string())?;

            let _ = VIEWPORT.insert(size);
        }
    
        Ok(())
    }

    pub fn run(self) -> Result<(), String> {
        let Self { mut inner, mut state, event_loop } = self;

        let err = rc::Rc::new(cell::OnceCell::new());
        let err_inner = rc::Rc::clone(&err);

        event_loop.run(move |event, event_target| {
            use winit::event::{Event, WindowEvent};

            if let Some(size) = unsafe { VIEWPORT.take() } {
                state.resize(size);
            }

            match event {
                Event::DeviceEvent { event, .. } => {
                    if inner.handle_event(event) {
                        state.window.request_redraw();
                    }
                },
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    window_id
                } if window_id == state.window.id() => {
                    inner.update(&state.queue);

                    if let Err(e) = state.submit_command_encoder({
                        |encoder, view| inner.submit_passes(encoder, view)
                    }) {
                        let _ = err_inner.get_or_init(|| e);

                        event_target.exit();
                    }
                },
                event => {
                    if let Err(e) = state.run(event, event_target) {
                        let _ = err_inner.get_or_init(|| e);
        
                        event_target.exit();
                    }
                },
            }
        }).map_err(|e| e.to_string())?;

        if let Some(mut container) = rc::Rc::into_inner(err) {
            if let Some(e) = container.take() { Err(e.to_string())?; }
        }

        Ok(())
    }
}

pub static mut VIEWPORT: Option<winit::dpi::PhysicalSize<u32>> = None;