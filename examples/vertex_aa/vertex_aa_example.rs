extern crate glutin;
extern crate gl;
extern crate vodk_gpu;
extern crate vodk_data;
extern crate vodk_math;
extern crate gfx2d;

use vodk_gpu::std140;
use vodk_gpu::device::*;
use vodk_gpu::constants::*;
use vodk_gpu::opengl;
use vodk_data as data;
use gfx2d::tesselation;
use gfx2d::color::Rgba;
use vodk_math::units::world;
use vodk_math::units::texels;
use vodk_math::matrix;

use std::mem;

#[derive(Debug)]
struct TransformsBlock {
  model: std140::Mat3,
  view:  std140::Mat3,
}

fn to_std_140_mat3<T>(from: &matrix::Matrix3x3<T>) -> std140::Mat3 {
    return std140::Mat3 {
        _11: from._11, _21: from._21, _31: from._31, _pad1: 0,
        _12: from._12, _22: from._22, _32: from._32, _pad2: 0,
        _13: from._13, _23: from._23, _33: from._33, _pad3: 0,
    }
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

    let path = &[
        world::vec2(-200.0,  -200.0),
        world::vec2(-200.0,   200.0),
        world::vec2( 200.0,   200.0),
        world::vec2( 200.0,  -200.0),
        world::vec2(  25.0,  -200.0),
        world::vec2(  25.0,  -100.0),
        world::vec2( -25.0,  -100.0),
        world::vec2( -25.0,  -200.0),
    ];

    let n_points = path.len();
    let indices_per_point = 18;
    let vertices_per_point = 4;
    let n_indices = indices_per_point * n_points;

    let vbo_desc = BufferDescriptor {
        size: (n_points * vertices_per_point *
              mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>()) as u32,
        buffer_type: BufferType::Vertex,
        update_hint: UpdateHint::Static,
    };

    let ibo_desc = BufferDescriptor {
        size: (mem::size_of::<u16>()  * n_indices) as u32,
        buffer_type: BufferType::Index,
        update_hint: UpdateHint::Static,
    };

    let vbo = ctx.create_buffer(&vbo_desc).ok().unwrap();
    let ibo = ctx.create_buffer(&ibo_desc).ok().unwrap();

    ctx.with_write_only_mapped_buffer(
      vbo, &|mapped_vbo| {
        tesselation::path_to_line_vbo(
            &path[..],
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            &|_| { 50.0 },
            &|_, ptype| { match ptype {
                tesselation::PointType::Antialias => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 0.0 },
                _ => Rgba { r: 0.0, g: 0.0, b: 0.3, a: 1.0 },
            }},
            world::Mat3::rotation(1.0),
            mapped_vbo
        );
      }
    );

    ctx.with_write_only_mapped_buffer(
      ibo, &|mapped_ibo| {
        tesselation::path_to_line_ibo(
            path.len() as u32,
            true,
            tesselation::VERTEX_ANTIALIASING|tesselation::CONVEX_SHAPE,
            0,
            mapped_ibo
        );
      }
    );

    let a_position = VertexAttributeLocation { index: 0 };
    let a_normal = VertexAttributeLocation { index: 1 };
    let a_color = VertexAttributeLocation { index: 2 };
    let a_extrusion = VertexAttributeLocation { index: 3 };

    let stride = mem::size_of::<tesselation::Pos2DNormal2DColorExtrusion>() as u16;
    let geom_desc = GeometryDescriptor{
        attributes: &[
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: a_position,
                stride: stride, offset: 0, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC2, location: a_normal,
                stride: stride, offset: 8, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::VEC4, location: a_color,
                stride: stride, offset: 16, normalize: false,
            },
            VertexAttribute {
                buffer: vbo,
                attrib_type: data::F32, location: a_extrusion,
                stride: stride, offset: 32, normalize: false,
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
        Err((_code, msg)) => { panic!("{}\nshader build failed - {}\n", shaders::VERTEX, msg); }
        _ => {}
    }

    let fragment_stage_desc = ShaderStageDescriptor {
        stage_type: ShaderType::Fragment,
        src: &[shaders::PIXEL]
    };
    let fragment_shader = ctx.create_shader_stage(&fragment_stage_desc).ok().unwrap();
    match ctx.get_shader_stage_result(fragment_shader) {
        Err((_code, msg)) => { panic!("{}\nshader build failed - {}\n", shaders::PIXEL, msg); }
        _ => {}
    }

    let pipeline_desc = ShaderPipelineDescriptor {
        stages: &[vertex_shader, fragment_shader],
        attrib_locations: &[
            ("a_position", a_position),
            ("a_normal", a_normal),
            ("a_color", a_color),
            ("a_extrusion", a_extrusion),
        ]
    };

    let pipeline = ctx.create_shader_pipeline(&pipeline_desc).ok().unwrap();

    match ctx.get_shader_pipeline_result(pipeline) {
        Err((_code, msg)) => { panic!("Shader link failed - {}\n", msg); }
        _ => {}
    }

    ctx.set_clear_color(0.9, 0.9, 0.9, 1.0);
    ctx.set_viewport(0, 0, win_width as i32, win_height as i32);

    let transforms_ubo_desc = BufferDescriptor {
        buffer_type: BufferType::Uniform,
        update_hint: UpdateHint::Dynamic,
        size: mem::size_of::<std140::Mat3>() as u32 * 2,
    };

    let static_ubo_desc = BufferDescriptor {
        buffer_type: BufferType::Uniform,
        update_hint: UpdateHint::Dynamic,
        size: mem::size_of::<texels::Vec2>() as u32,
    };

    let transforms_ubo = ctx.create_buffer(&transforms_ubo_desc).ok().unwrap();
    ctx.with_write_only_mapped_buffer::<TransformsBlock>(
      transforms_ubo, &|mapped_data| {
        mapped_data[0].model = to_std_140_mat3(&world::Mat3::identity());
        mapped_data[0].view = to_std_140_mat3(&world::Mat3::identity());
      }
    );

    let static_ubo = ctx.create_buffer(&static_ubo_desc).ok().unwrap();
    ctx.with_write_only_mapped_buffer::<texels::Vec2>(
      static_ubo, &|mapped_data| {
        mapped_data[0].x = win_width as f32;
        mapped_data[0].y = win_height as f32;
      }
    );

    let transforms_binding_index = 0;
    let static_binding_index = 1;

    ctx.bind_uniform_buffer(transforms_binding_index, transforms_ubo, None);
    let u_transforms = ctx.get_uniform_block_location(pipeline, "u_transforms");
    assert!(u_transforms.index >= 0);
    ctx.set_uniform_block(pipeline, u_transforms, transforms_binding_index);

    ctx.bind_uniform_buffer(static_binding_index, static_ubo, None);
    let u_static = ctx.get_uniform_block_location(pipeline, "u_static");
    assert!(u_static.index >= 0);
    ctx.set_uniform_block(pipeline, u_static, static_binding_index);

    ctx.set_shader(pipeline);

    while !window.should_close() {
        ctx.clear(COLOR|DEPTH);
        ctx.draw(
            geom,
            Range::IndexRange(0, n_indices as u16),
            TRIANGLES, BlendMode::None, COLOR|DEPTH
        );

        window.swap_buffers();
    }
}

pub mod shaders {
pub const VERTEX: &'static str = "
#version 140
attribute vec2 a_position;
attribute vec2 a_normal;
attribute vec4 a_color;
attribute float a_extrusion;

uniform u_static {
    vec2 resolution;
};
uniform u_transforms {
    mat3 model;
    mat3 view;
};

varying vec4 v_color;

void main() {
  mat3 transform = model;// * model;
  float scale = length(transform * vec3(1.0,0.0,0.0));
  vec2 pos = (a_position + a_normal * a_extrusion / scale) / resolution;
  gl_Position = vec4(transform * vec3(pos, 0.0), 1.0);
  v_color = a_color;
}
";

pub static PIXEL: &'static str = "
varying vec4 v_color;
void main() {
    gl_FragColor = v_color;
}
";
}

