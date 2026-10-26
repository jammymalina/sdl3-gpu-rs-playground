use std::time::Instant;

use sdl3::{event::Event, keyboard::Keycode};
use sdl3_gpu_rs_playground::{
    assets::{MaterialRepository, MeshRepository},
    camera::{Camera, PerspectiveCamera, PerspectiveCameraConfig},
    geom,
    gpu::allocator::MemAllocator,
    light::{DirectionalLight, MainLight},
    render::{context::RenderContextConfig, renderer::Renderer},
    scene::Scene,
};

pub fn main() -> anyhow::Result<()> {
    const MS_PER_UPDATE: u128 = 16;

    let renderer = Renderer::new(&RenderContextConfig {
        title: "Playground".into(),
        window_width: 800,
        window_height: 600,
    })?;

    let mut scene = Scene::default();
    scene.set_main_light(MainLight::Directional(DirectionalLight::default()));

    let mut camera = PerspectiveCamera::new(PerspectiveCameraConfig {
        vertical_fov: 75.0,
        aspect_ratio: 1.0,
        near: 0.1,
        far: 1000.0,
        ..Default::default()
    });
    camera.transform_mut().position[2] = -5.0;

    let allocator = MemAllocator::new(&renderer.context().device);
    let mut mesh_repo = MeshRepository::new(&allocator);
    let mut material_repo = MaterialRepository::default();

    let cube_geom = geom::cuboid::cuboid(geom::cuboid::CuboidConfig::default());

    let mut event_pump = renderer.context().sdl_context.event_pump()?;

    let mut previous = Instant::now();
    let mut lag = 0;
    'main: loop {
        let current = Instant::now();
        let elapsed = current - previous;
        previous = current;
        lag += elapsed.as_millis();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | sdl3::event::Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }

        // Update scene
        while lag >= MS_PER_UPDATE {
            lag -= MS_PER_UPDATE;
        }

        renderer.render(&scene, &mesh_repo, &material_repo)?;
    }

    Ok(())
}
