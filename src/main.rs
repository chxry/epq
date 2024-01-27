use std::{mem, slice};
use winit::event_loop::EventLoop;
use winit::event::{Event, WindowEvent};
use winit::window::WindowBuilder;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::filter::LevelFilter;
use glam::UVec2;
use shared::Consts;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::INFO))
    .init();
  let event_loop = EventLoop::new()?;
  let window = WindowBuilder::new().build(&event_loop)?;

  let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
  let surface = instance.create_surface(&window)?;
  let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::HighPerformance,
      compatible_surface: Some(&surface),
      force_fallback_adapter: false,
    })
    .await
    .unwrap();
  let (device, queue) = adapter
    .request_device(
      &wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        required_limits: wgpu::Limits::default(),
        label: None,
      },
      None,
    )
    .await?;

  let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
      ty: wgpu::BindingType::StorageTexture {
        access: wgpu::StorageTextureAccess::ReadWrite,
        format: wgpu::TextureFormat::Rgba32Float,
        view_dimension: wgpu::TextureViewDimension::D2,
      },
      count: None,
    }],
  });
  let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
  });
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[&tex_layout, &uniform_layout],
    push_constant_ranges: &[],
  });

  let shader = device.create_shader_module(wgpu::include_spirv!(env!("shaders.spv")));
  let rt_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    layout: Some(&pipeline_layout),
    module: &shader,
    entry_point: "rt::main",
    label: None,
  });
  let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: "quad_v",
      buffers: &[],
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: "test_f",
      targets: &[Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
      })],
    }),
    primitive: wgpu::PrimitiveState::default(),
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
    label: None,
  });

  let mut framebuffer = Framebuffer::new(&device, 1, 1, &tex_layout);
  let mut uniform = Uniform::new(&device, &uniform_layout);

  let window = &window;
  event_loop.run(move |event, elwt| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => {
        surface.configure(
          &device,
          &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 1,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
          },
        );
        framebuffer = Framebuffer::new(&device, size.width, size.height, &tex_layout);
        uniform.data.size = UVec2::new(size.width, size.height);
        uniform.data.samples = 1;
      }
      WindowEvent::RedrawRequested => {
        let surface = surface.get_current_texture().unwrap();
        let surface_view = surface
          .texture
          .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        uniform.update(&queue);

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
          label: None,
          timestamp_writes: None,
        });
        compute_pass.set_pipeline(&rt_pipeline);
        compute_pass.set_bind_group(0, &framebuffer.bind_group, &[]);
        compute_pass.set_bind_group(1, &uniform.bind_group, &[]);
        compute_pass.dispatch_workgroups(framebuffer.tex.width(), framebuffer.tex.height(), 1);
        uniform.data.samples += 1;
        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &surface_view,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Load,
              store: wgpu::StoreOp::Store,
            },
          })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
          label: None,
        });
        render_pass.set_pipeline(&ui_pipeline);
        render_pass.set_bind_group(0, &framebuffer.bind_group, &[]);
        render_pass.set_bind_group(1, &uniform.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        queue.submit([encoder.finish()]);
        surface.present();
      }
      WindowEvent::CloseRequested => elwt.exit(),
      _ => {}
    },
    Event::AboutToWait => window.request_redraw(),
    _ => {}
  })?;
  Ok(())
}

struct Framebuffer {
  tex: wgpu::Texture,
  bind_group: wgpu::BindGroup,
}

impl Framebuffer {
  fn new(device: &wgpu::Device, width: u32, height: u32, layout: &wgpu::BindGroupLayout) -> Self {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba32Float,
      usage: wgpu::TextureUsages::STORAGE_BINDING,
      view_formats: &[],
      label: None,
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&view),
      }],
      label: None,
    });
    Self { tex, bind_group }
  }
}

struct Uniform {
  data: Consts,
  buf: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
}

impl Uniform {
  fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
    let data = Consts::default();
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
      size: mem::size_of::<Consts>() as _,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
      label: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: buf.as_entire_binding(),
      }],
      label: None,
    });
    Self {
      data,
      buf,
      bind_group,
    }
  }

  fn update(&self, queue: &wgpu::Queue) {
    queue.write_buffer(&self.buf, 0, cast(&self.data));
  }
}

fn cast_slice<T>(t: &[T]) -> &[u8] {
  // safety: u8 is always valid
  unsafe { slice::from_raw_parts(t.as_ptr() as _, mem::size_of_val(t)) }
}

fn cast<T>(t: &T) -> &[u8] {
  cast_slice(slice::from_ref(t))
}
