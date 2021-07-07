#[macro_use]
extern crate glium;
extern crate nalgebra;

use std::env::Args;
use std::error::Error;
use std::fs;

pub mod parser;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    run_gl(config.filename)?;

    Ok(())
}

fn run_gl(filename: String) -> Result<(), Box<dyn Error>> {
    #[allow(unused_imports)]
    use glium::{glutin, Surface};
    use nalgebra::{Matrix4, Vector3};

    let width = 500.0f32;
    let height = 500.0f32;

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(width, height));
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    #[derive(Copy, Clone, Debug)]
    struct Vertex {
        position: [f32; 3],
    }

    implement_vertex!(Vertex, position);

    let content = fs::read_to_string(filename)?;
    let object = parser::parse_obj_threaded(content).unwrap();

    let positions = object.position;

    let shape = positions
        .chunks(3)
        .map(|position| Vertex {
            position: [position[0] as f32, position[1] as f32, position[2] as f32],
        })
        .collect::<Vec<_>>();

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 140
        in vec3 position;
        uniform mat4 matrix;
        void main() {
            gl_Position = matrix * vec4(position, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140
        out vec4 color;
        void main() {
            color = vec4(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let program =
        glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None)
            .unwrap();

    let projection_matrix = Matrix4::new_perspective(width / height, 90.0, 0.1, 100.0);
    let translation = Matrix4::new_translation(&Vector3::new(0.0, 0.0, -10.0));
    let scale = Matrix4::from_scaled_axis(Vector3::new(1.0, 1.0, 1.0));

    let mut t: f32 = -0.5;
    event_loop.run(move |event, _, control_flow| {
        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                }
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        let next_frame_time =
            std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        // we update `t`
        t += 0.02;
        t %= 6.28;

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);

        let rotation = Matrix4::new_rotation(Vector3::new(0.0, 1.0, 0.0) * t);

        let matrix = projection_matrix * translation * rotation * scale;

        let uniforms = uniform! {
            matrix: [
                [matrix[0], matrix[1], matrix[2], matrix[3]],
                [matrix[4], matrix[5], matrix[6], matrix[7]],
                [matrix[8], matrix[9], matrix[10], matrix[11]],
                [matrix[12], matrix[13], matrix[14], matrix[15]],
            ],
        };

        target
            .draw(
                &vertex_buffer,
                &indices,
                &program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();
    });
}

pub struct Config {
    filename: String,
}

impl Config {
    pub fn new(mut args: Args) -> Result<Config, &'static str> {
        args.next();

        let filename = match args.next() {
            Some(arg) => arg,
            None => return Err("Filename not specified."),
        };

        Ok(Config { filename })
    }
}
