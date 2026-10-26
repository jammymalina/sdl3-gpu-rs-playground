use std::time::Instant;

use sdl3::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
};
use sdl3_gpu_rs_playground::{
    assets::{MaterialRepository, MeshRepository},
    camera::{Camera, PerspectiveCamera, PerspectiveCameraConfig},
    geom,
    gpu::allocator::MemAllocator,
    light::{DirectionalLight, MainLight},
    material::{BasicColor, Material},
    render::{context::RenderContextConfig, renderer::Renderer},
    scene::{Node, Scene},
};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub fn main() -> anyhow::Result<()> {
    const MS_PER_UPDATE: u16 = 16;
    const SPIN_SPEED: f32 = 1.0;
    const TILT_SPEED: f32 = 0.4;
    let dt = f32::from(MS_PER_UPDATE) / 1000.0;

    let renderer = Renderer::new(&RenderContextConfig {
        title: "Playground".into(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
    })?;

    let mut scene = Scene::default();
    scene.set_main_light(MainLight::Directional(DirectionalLight::default()));
    scene.set_clear_color([0.07, 0.05, 0.04, 1.0]);

    let mut camera = PerspectiveCamera::new(PerspectiveCameraConfig {
        vertical_fov: 75.0_f32.to_radians(),
        near: 0.1,
        far: 1000.0,
        ..Default::default()
    });
    let (width, height) = renderer.context().window.size_in_pixels();
    camera.set_view_size(cast::f32(width), cast::f32(height));
    camera.transform_mut().position[2] = -3.0;

    let allocator = MemAllocator::new(&renderer.context().device);
    let mut mesh_repo = MeshRepository::new(&allocator);
    let mut material_repo = MaterialRepository::default();

    let cube_mesh = {
        let cube_geom = geom::cuboid::cuboid(geom::cuboid::CuboidConfig::default());
        mesh_repo.add_mesh_from_mesh_data(&cube_geom)?
    };
    let flat_orange_mat = material_repo.insert(Material::FlatColor {
        basic_color: BasicColor {
            base_color: [0.9, 0.45, 0.15, 1.0],
            ambient_color: [0.12, 0.05, 0.02, 1.0],
        },
    });

    let mut cube_node = Node::new("cube");
    cube_node.mesh = Some(cube_mesh);
    cube_node.material = Some(flat_orange_mat);
    let cube_key = scene.spawn_node(cube_node);

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
                Event::Window {
                    win_event: WindowEvent::PixelSizeChanged(width, height),
                    ..
                } => camera.set_view_size(cast::f32(width), cast::f32(height)),
                _ => {}
            }
        }

        // Update scene
        while lag >= u128::from(MS_PER_UPDATE) {
            lag -= u128::from(MS_PER_UPDATE);

            if let Some(cube) = scene.get_mut(cube_key) {
                cube.transform.rotate_y(SPIN_SPEED * dt);
                cube.transform.rotate_x(TILT_SPEED * dt);
            }
        }
        scene.update_world_matrix();

        renderer.render(&scene, &camera, &mesh_repo, &material_repo)?;
    }

    Ok(())
}
