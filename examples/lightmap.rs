//! Example 06. Lightmap.
//!
//! Difficulty: Easy.
//!
//! This example shows how to load simple scene made in [rusty-editor](https://github.com/mrDIMAS/rusty-editor)
//! and generate lightmap for it. Lightmaps are still in active development and not meant to use.

extern crate rg3d;

use rg3d::gui::message::MessageDirection;
use rg3d::{
    core::{
        color::Color,
        math::{quat::Quat, vec3::Vec3},
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{message::TextMessage, node::StubNode, text::TextBuilder, widget::WidgetBuilder},
    scene::{
        base::BaseBuilder, camera::CameraBuilder, node::Node, transform::TransformBuilder, Scene,
    },
    utils::{lightmap::Lightmap, translate_event, uvgen},
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

// Create our own engine type aliases. These specializations are needed
// because engine provides a way to extend UI with custom nodes and messages.
type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type BuildContext<'a> = rg3d::gui::BuildContext<'a, (), StubNode>;

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

struct GameScene {
    scene: Scene,
    root: Handle<Node>,
}

fn create_scene(resource_manager: Arc<Mutex<ResourceManager>>) -> GameScene {
    let mut scene = Scene::new();

    let mut resource_manager = resource_manager.lock().unwrap();

    // Camera is our eyes in the world - you won't see anything without it.
    let camera = CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vec3::new(4.0, 4.0, -8.0))
                .build(),
        ),
    )
    .build();

    scene.graph.add_node(Node::Camera(camera));

    // There is no difference between scene created in rusty-editor and any other
    // model file, so any scene can be used directly as resource.
    let root = resource_manager
        .request_model("examples/data/test_scene.rgs")
        .unwrap()
        .lock()
        .unwrap()
        .instantiate(&mut scene)
        .root;
    scene.graph.update_hierachical_data();

    for node in scene.graph.linear_iter() {
        if let Node::Mesh(mesh) = node {
            uvgen::generate_uvs_mesh(mesh, 0.02);
        }
    }

    let lightmap = Lightmap::new(&scene, 128);
    lightmap.save("examples/data/lightmaps/").unwrap();
    scene.set_lightmap(lightmap).unwrap();

    for node in scene.graph.linear_iter_mut() {
        if let Node::Light(_) = node {
            node.set_visibility(false);
        }
    }

    GameScene { scene, root }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("Example - Lightmap")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

    // Prepare resource manager - it must be notified where to search textures. When engine
    // loads model resource it automatically tries to load textures it uses. But since most
    // model formats store absolute paths, we can't use them as direct path to load texture
    // instead we telling engine to search textures in given folder.
    engine
        .resource_manager
        .lock()
        .unwrap()
        .set_textures_path("examples/data");

    // Create simple user interface that will show some useful info.
    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    // Create test scene.
    let GameScene { scene, root } = create_scene(engine.resource_manager.clone());

    // Add scene to engine - engine will take ownership over scene and will return
    // you a handle to scene which can be used later on to borrow it and do some
    // actions you need.
    let scene_handle = engine.scenes.add(scene);

    // Set ambient light.
    engine
        .renderer
        .set_ambient_color(Color::opaque(200, 200, 200));

    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = 180.0f32.to_radians();

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController {
        rotate_left: false,
        rotate_right: false,
    };

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly. Engine lets you to decide which event should be handled,
    // this is minimal working example if how it should be.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;

                    // ************************
                    // Put your game logic here.
                    // ************************

                    // Use stored scene handle to borrow a mutable reference of scene in
                    // engine.
                    let scene = &mut engine.scenes[scene_handle];

                    // Rotate model according to input controller state.
                    if input_controller.rotate_left {
                        model_angle -= 5.0f32.to_radians();
                    } else if input_controller.rotate_right {
                        model_angle += 5.0f32.to_radians();
                    }

                    scene.graph[root]
                        .local_transform_mut()
                        .set_rotation(Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), model_angle));

                    let fps = engine.renderer.get_statistics().frames_per_second;
                    let text = format!(
                        "Example 06 - Lightmap\nUse [A][D] keys to rotate scene.\nFPS: {}",
                        fps
                    );
                    engine.user_interface.send_message(TextMessage::text(
                        debug_text,
                        MessageDirection::ToWidget,
                        text,
                    ));

                    engine.update(fixed_timestep);
                }

                // It is very important to "pump" messages from UI. Even if don't need to
                // respond to such message, you should call this method, otherwise UI
                // might behave very weird.
                while let Some(_ui_event) = engine.user_interface.poll_message() {
                    // ************************
                    // Put your data model synchronization code here. It should
                    // take message and update data in your game according to
                    // changes in UI.
                    // ************************
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                engine.get_window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                engine.render(fixed_timestep).unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.renderer.set_frame_size(size.into());
                    }
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::Key(key) = event {
                    if let Some(key_code) = key.virtual_keycode {
                        match key_code {
                            VirtualKeyCode::A => {
                                input_controller.rotate_left = key.state == ElementState::Pressed
                            }
                            VirtualKeyCode::D => {
                                input_controller.rotate_right = key.state == ElementState::Pressed
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
