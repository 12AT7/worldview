use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
    time::Instant,
};

use tokio::sync::watch;
use wgpu;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    pipeline, Artifact, ArtifactUniform, Camera, CameraController, CameraUniform, PlaybackEvent,
    PlaybackLock, Projection, RenderArtifact,
};

// The playback thread needs to load GPU buffers, and for that it
// needs the device and queue from the wgpu state.  Because threads
// can only take 'static lifetime references, and we don't even have a
// wgpu surface until the window is created, sharing these references
// is a PITA.  They are not Serialize either, so we cannot even use an
// async channel to message between threads.  So, let's use global
// variables so any thread can get these critical objects.
pub static DEVICE: OnceLock<wgpu::Device> = OnceLock::new();
pub static QUEUE: OnceLock<wgpu::Queue> = OnceLock::new();

pub struct WindowState<'win> {
    surface: wgpu::Surface<'win>,
    window: &'win Window,
    playback: PlaybackLock,
    exit: watch::Sender<bool>,
    pub surface_capabilities: wgpu::SurfaceCapabilities,
    pub point_cloud_pipeline_layout: wgpu::PipelineLayout,
    pub wireframe_pipeline_layout: wgpu::PipelineLayout,
    pub mesh_pipeline_layout: wgpu::PipelineLayout,
    world_bind_group_layout: wgpu::BindGroupLayout,
    artifact_bind_group_layout: wgpu::BindGroupLayout,
    pub world_bind_group: wgpu::BindGroup,
    pipeline: HashMap<String, wgpu::RenderPipeline>,
    artifact_bind_group: HashMap<String, wgpu::BindGroup>,
    artifact_uniform_buffer: HashMap<String, wgpu::Buffer>,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_uniform: CameraUniform,
    camera_controller: CameraController,
    projection: Projection,
    mouse_pressed: bool,
}

impl<'win> WindowState<'win> {
    pub async fn new(
        window: &'win Window,
        playback: PlaybackLock,
        exit: watch::Sender<bool>,
    ) -> WindowState<'win> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();

        let camera = Camera::new();
        let projection = Projection::new(size.width, size.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = CameraController::new();

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // let artifact_uniform = ArtifactUniform::new();
        // let artifact_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Model Uniform Buffer"),
        //     contents: bytemuck::cast_slice(&[artifact_uniform]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });

        let world_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // CameraUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("uniform_bind_group_layout"),
            });

        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &world_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("world_bind_group"),
        });

        let artifact_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // ArtifactUniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("artifact_bind_group_layout"),
            });

        let point_cloud_pipeline_layout = pipeline::PointCloud::create_pipeline_layout(
            &device,
            &world_bind_group_layout,
            &artifact_bind_group_layout,
        );

        let wireframe_pipeline_layout = pipeline::Wireframe::create_pipeline_layout(
            &device,
            &world_bind_group_layout,
            &artifact_bind_group_layout,
        );

        let mesh_pipeline_layout = pipeline::Mesh::create_pipeline_layout(
            &device,
            &world_bind_group_layout,
            &artifact_bind_group_layout,
        );

        DEVICE.set(device).unwrap();
        QUEUE.set(queue).unwrap();

        WindowState {
            surface,
            window,
            playback,
            exit,
            surface_capabilities,
            point_cloud_pipeline_layout,
            wireframe_pipeline_layout,
            mesh_pipeline_layout,
            world_bind_group_layout,
            artifact_bind_group_layout,
            world_bind_group,
            pipeline: HashMap::new(),
            artifact_bind_group: HashMap::new(),
            artifact_uniform_buffer: HashMap::new(),
            camera,
            camera_buffer,
            camera_uniform,
            camera_controller,
            projection,
            mouse_pressed: false,
        }
    }

    fn resize(&self, size: dpi::PhysicalSize<u32>) {
        let format = self.surface_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format],
            desired_maximum_frame_latency: 2,
        };

        let device = DEVICE.get().unwrap();
        self.surface.configure(&device, &config);
    }

    fn redraw(&mut self) {
        let playback = self.playback.lock().unwrap();

        let device = match DEVICE.get() {
            Some(device) => device,
            None => {
                log::debug!("Playback waiting for WGPU initialization");
                return;
            }
        };

        for (key, artifact) in &playback.artifact {
            if !self.pipeline.contains_key(key) {
                let pipeline = artifact.create_pipeline(&device, &self);
                let buffer = artifact.create_uniform_buffer(&device);
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.artifact_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                    label: Some("artifact_bind_group"),
                });

                self.pipeline.insert(key.clone(), pipeline);
                self.artifact_bind_group.insert(key.clone(), bind_group);
                self.artifact_uniform_buffer.insert(key.clone(), buffer);
            }
        }

        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        let surface = &self.surface;
        let output = match surface.get_current_texture() {
            Ok(surface) => surface,
            Err(e) => {
                log::error!("surface {:?}", e);
                return;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_bind_group(0, &self.world_bind_group, &[]);
            for (key, artifact) in &playback.artifact {
                render_pass.set_pipeline(self.pipeline.get(key).unwrap());
                render_pass.set_bind_group(1, &self.artifact_bind_group.get(key).unwrap(), &[]);
                match artifact {
                    Artifact::PointCloud(point_cloud) => {
                        pipeline::PointCloud::render(
                            &point_cloud.vertices,
                            &self,
                            &mut render_pass,
                        );
                    }
                    Artifact::Wireframe(wireframe) => {
                        pipeline::Wireframe::render(
                            &wireframe.vertices,
                            &wireframe.indices,
                            &self,
                            &mut render_pass,
                        );
                    }
                    Artifact::Mesh(mesh) => {
                        pipeline::Mesh::render(
                            &mesh.vertices,
                            &mesh.indices,
                            &self,
                            &mut render_pass,
                        );
                    }
                }
            }
        }

        // Let 'er rip.  Render the frame.
        let queue = QUEUE.get().unwrap();

        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        queue.submit([encoder.finish()]);
        output.present();
    }
}

impl<'win> ApplicationHandler<PlaybackEvent> for WindowState<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: PlaybackEvent) {
        match event {
            PlaybackEvent::Refresh(key) => {
                self.window.request_redraw();
            }
            _ => {
                log::info!("Unhandled user event: {event:?}");
            }
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device: DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if self.mouse_pressed {
                    self.camera_controller.process_mouse(delta.0, delta.1);
                    self.camera_controller.update_camera(&mut self.camera);
                    self.camera_uniform
                        .update_view_proj(&self.camera, &self.projection);
                }
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
                self.exit.send(true).unwrap();
            }
            WindowEvent::Resized(size) => {
                self.resize(size);
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = state == ElementState::Pressed;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
            }
            _ => {}
        }
    }
}

pub async fn run(
    playback: PlaybackLock,
    event_loop: EventLoop<PlaybackEvent>,
    exit: watch::Sender<bool>,
) {
    let window = event_loop
        .create_window(WindowAttributes::default())
        .unwrap();

    let mut app = WindowState::new(&window, playback.clone(), exit).await;
    let _ = event_loop.run_app(&mut app);
}
