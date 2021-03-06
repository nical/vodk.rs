extern crate glutin;
extern crate gl;
extern crate vodk_gpu;
extern crate vodk_data;
extern crate vodk_math;

use vodk_gpu::device::*;
use vodk_gpu::constants::*;
use vodk_gpu::opengl;
use vodk_data as data;

use std::num::Float;

use vodk_math::units::world;

struct TransformsBlock {
  model: world::Mat4,
  view: world::Mat4,
  projection: world::Mat4,
}

fn main() {
    let win_width: u32 = 800;
    let win_height: u32 = 600;

    let window = glutin::WindowBuilder::new()
        .with_title(format!("Vertex AA test"))
        .with_dimensions(800,600)
        .with_gl_version((3,3))
        .with_vsync()
        .build().unwrap();

    unsafe { window.make_current() };

    gl::load_with(|symbol| window.get_proc_address(symbol));

    let mut ctx = opengl::create_debug_device(LOG_ERRORS|CRASH_ERRORS);

    let cube_vertices: &[f32] = &[
        // Front face     |     normals     | tex coords
        -1.0, -1.0, 1.0,    0.0, 0.0, 1.0,    1.0, 0.0,
         1.0, -1.0, 1.0,    0.0, 0.0, 1.0,    1.0, 1.0,
         1.0,  1.0, 1.0,    0.0, 0.0, 1.0,    0.0, 1.0,
        -1.0,  1.0, 1.0,    0.0, 0.0, 1.0,    0.0, 0.0,
        // Back face
        -1.0, -1.0, -1.0,   0.0, 0.0, -1.0,   1.0, 0.0,
        -1.0,  1.0, -1.0,   0.0, 0.0, -1.0,   1.0, 1.0,
         1.0,  1.0, -1.0,   0.0, 0.0, -1.0,   0.0, 1.0,
         1.0, -1.0, -1.0,   0.0, 0.0, -1.0,   0.0, 0.0,
        // Top face
        -1.0, 1.0, -1.0,    0.0, 1.0, 1.0,    1.0, 0.0,
        -1.0, 1.0,  1.0,    0.0, 1.0, 1.0,    1.0, 1.0,
         1.0, 1.0,  1.0,    0.0, 1.0, 1.0,    0.0, 1.0,
         1.0, 1.0, -1.0,    0.0, 1.0, 1.0,    0.0, 0.0,
        // Bottom face
        -1.0, -1.0, -1.0,   0.0, -1.0, 1.0,   1.0, 0.0,
         1.0, -1.0, -1.0,   0.0, -1.0, 1.0,   1.0, 1.0,
         1.0, -1.0,  1.0,   0.0, -1.0, 1.0,   0.0, 1.0,
        -1.0, -1.0,  1.0,   0.0, -1.0, 1.0,   0.0, 0.0,
        // Right face
         1.0, -1.0, -1.0,   1.0, 0.0, 1.0,    1.0, 0.0,
         1.0,  1.0, -1.0,   1.0, 0.0, 1.0,    1.0, 1.0,
         1.0,  1.0,  1.0,   1.0, 0.0, 1.0,    0.0, 1.0,
         1.0, -1.0,  1.0,   1.0, 0.0, 1.0,    0.0, 0.0,
        // Left face
        -1.0, -1.0, -1.0,   -1.0, 0.0, 1.0,   1.0, 0.0,
        -1.0, -1.0,  1.0,   -1.0, 0.0, 1.0,   1.0, 1.0,
        -1.0,  1.0,  1.0,   -1.0, 0.0, 1.0,   0.0, 1.0,
        -1.0,  1.0, -1.0,   -1.0, 0.0, 1.0,   0.0, 0.0
    ];

    let cube_indices: &[u16] = &[
        0, 1, 2, 0, 2, 3,         // Front face
        4, 5, 6, 4, 6, 7,         // Back face
        8, 9, 10, 8, 10, 11,      // Top face
        12, 13, 14, 12, 14, 15,   // Bottom face
        16, 17, 18, 16, 18, 19,   // Right face
        20, 21, 22, 20, 22, 23    // Left face
    ];

    let vbo_desc = BufferDescriptor {
        size: 8*4*4*6,
        buffer_type: BufferType::Vertex,
        update_hint: UpdateHint::Static,
    };

    let ibo_desc = BufferDescriptor {
        size: 8*4*4*6,
        buffer_type: BufferType::Index,
        update_hint: UpdateHint::Static,
    };

    let vbo = ctx.create_buffer(&vbo_desc).ok().unwrap();
    let ibo = ctx.create_buffer(&ibo_desc).ok().unwrap();

    ctx.with_write_only_mapped_buffer(
      vbo, &|mapped_vbo| {
          for i in 0 .. cube_vertices.len() {
            mapped_vbo[i] = cube_vertices[i];
          }
      }
    );

    ctx.with_write_only_mapped_buffer(
      ibo, &|mapped_ibo| {
          for i in 0 .. cube_indices.len() {
            mapped_ibo[i] = cube_indices[i];
          }
      }
    );

    let a_position = VertexAttributeLocation { index: 0 };
    let a_normal = VertexAttributeLocation { index: 1 };
    let a_tex_coords = VertexAttributeLocation { index: 2 };

    let geom_desc = GeometryDescriptor{
      attributes: &[
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC3,
            location: a_position,
            stride: 32,
            offset: 0,
            normalize: false,
        },
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC3,
            location: a_normal,
            stride: 32,
            offset: 12,
            normalize: false,
        },
        VertexAttribute {
            buffer: vbo,
            attrib_type: data::VEC2,
            location: a_tex_coords,
            stride: 32,
            offset: 24,
            normalize: false,
        }
      ],
      index_buffer: Some(ibo)
    };

    let geom = ctx.create_geometry(&geom_desc).ok().unwrap();

    let vertex_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::Vertex,
        src: &[shaders::VERTEX]
    };

    let vertex_shader = ctx.create_shader_stage(&vertex_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(vertex_shader) {
        Err((_code, msg)) => {
            panic!("{}\nshader build failed - {}\n", shaders::VERTEX, msg);
        }
        _ => {}
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::Fragment,
        src: &[shaders::FRAGMENT]
    };
    let fragment_shader = ctx.create_shader_stage(&fragment_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(fragment_shader) {
        Err((_code, msg)) => {
            panic!("{}\nshader build failed - {}\n", shaders::FRAGMENT, msg);
        }
        _ => {}
    }

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[vertex_shader, fragment_shader],
        attrib_locations: &[
            ("a_position", a_position),
            ("a_normal", a_normal),
            ("a_uv_tex_coords", a_tex_coords),
        ]
    };

    let pipeline = ctx.create_shader_pipeline(&pipeline_desc).ok().unwrap();

    match ctx.get_shader_pipeline_result(pipeline) {
        Err((_code, msg)) => {
            panic!("Shader link failed - {}\n", msg);
        }
        _ => {}
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, win_width as i32, win_height as i32);

    let ubo_desc = BufferDescriptor {
        buffer_type: BufferType::Uniform,
        update_hint: UpdateHint::Dynamic,
        size: 4*16*3,
    };

    let ubo = ctx.create_buffer(&ubo_desc).ok().unwrap();

    let ubo_binding_index = 0;
    ctx.bind_uniform_buffer(ubo_binding_index, ubo, None);
    let u_transforms = ctx.get_uniform_block_location(pipeline, "u_transforms");
    assert!(u_transforms.index >= 0);
    ctx.set_uniform_block(pipeline, u_transforms, ubo_binding_index);

    let mut frame_count: u64 = 0;
    while !window.should_close() {
        frame_count+=1;
        ctx.with_write_only_mapped_buffer::<TransformsBlock>(
            ubo, &|mapped_ubo| {
                let mut perspective_mat = world::Mat4::identity();
                world::Mat4::perspective(
                    45.0,
                    win_width as f32 / win_height as f32,
                    0.5,
                    1000.0,
                    &mut perspective_mat
                );
                mapped_ubo[0].projection = perspective_mat;
                let view = &mut mapped_ubo[0].view;
                *view = world::Mat4::identity();
                view.translate(&world::vec3(0.0, 0.0, -10.0));
                view.rotate(
                    3.14159265358979323846264338327950288 * (frame_count as f32 * 0.01).sin(),
                    &world::vec3(0.0, 1.0, 0.0)
                );
                mapped_ubo[0].model = world::Mat4::identity();
            }
        );

        ctx.clear(COLOR|DEPTH);
        ctx.set_shader(pipeline);
        ctx.draw(
            geom,
            Range::IndexRange(0, cube_indices.len() as u16),
            TRIANGLES, BlendMode::None, COLOR|DEPTH
        );

        window.swap_buffers();
    }
}

pub mod shaders {
pub const VERTEX: &'static str = "
#version 150
layout(std140)
uniform u_transforms {
  mat4 model;
  mat4 view;
  mat4 projection;
};
attribute vec3 a_position;
attribute vec3 a_normal;
attribute vec2 a_tex_coords;
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    v_tex_coords = a_tex_coords;
    v_normal = a_normal;
    gl_Position = projection
                * view
                * model
                * vec4(a_position, 1.0);
}
";

pub static FRAGMENT: &'static str = "
varying vec3 v_normal;
varying vec2 v_tex_coords;
void main() {
    vec3 normals = v_normal * 0.5 + vec3(0.5, 0.5, 0.5);
    gl_FragColor = vec4(normals, 1.0);
}
";
}
