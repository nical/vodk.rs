#[cfg(test)]
mod test {

use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gfx::renderer;
use gfx::shaders;

#[test]
pub fn test() {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::ContextVersion(3, 1));
    glfw.window_hint(glfw::OpenglForwardCompat(true));

    let (window, _) = glfw.create_window(800, 600, "OpenGL", glfw::Windowed)
        .expect("Failed to create GLFW window.");

    window.make_current();

    gl::load_with(|s| glfw.get_proc_address(s));

    let mut gl = opengl::RenderingContextGL::new();
    let mut ctx = &mut gl as &mut renderer::RenderingContext;
    test_texture_upload_readback(ctx);
    ctx.reset_state();
    test_render_to_texture(ctx);
    ctx.reset_state();

    window.swap_buffers();
}

fn test_texture_upload_readback(ctx: &mut renderer::RenderingContext) {
    println!("test test_texture_upload_readback...");
    let checker_data : Vec<u8> = Vec::from_fn(64*64*4, |i|{ (((i / 4) % 2)*255) as u8 });

    let checker = ctx.create_texture(renderer::REPEAT|renderer::FILTER_NEAREST);

    ctx.upload_texture_data(checker, checker_data.as_slice(), 64, 64, renderer::R8G8B8A8);

    let mut checker_read_back : Vec<u8> = Vec::from_fn(64*64*4, |i|{ 1 as u8 });

    assert!(checker_data != checker_read_back);

    ctx.read_back_texture(checker, renderer::R8G8B8A8,
                          checker_read_back.as_mut_slice());

    assert_eq!(checker_data, checker_read_back);

    ctx.destroy_texture(checker);
}

fn test_render_to_texture(ctx: &mut renderer::RenderingContext) {
    println!("test test_render_to_texture...");
    let w = 256;
    let h = 256;

    ctx.set_clear_color(0.0, 1.0, 0.0, 1.0);

    let target_texture = ctx.create_texture(renderer::CLAMP|renderer::FILTER_NEAREST);
    ctx.allocate_texture(target_texture, w, h, renderer::R8G8B8A8);
    let rt = match ctx.create_render_target([target_texture], None, None) {
        Ok(target) => target,
        Err(_) => fail!()
    };

    ctx.set_render_target(rt);

    ctx.clear(renderer::COLOR);

    let mut read_back : Vec<u8> = Vec::from_fn((w*h*4) as uint, |i|{ 1 as u8 });
    ctx.read_back_texture(target_texture, renderer::R8G8B8A8,
                          read_back.as_mut_slice());

    for j in range(0, h) {
        for i in range(0, w) {
            assert_eq!(*read_back.get(((i+j*h)*4    ) as uint), 0);
            assert_eq!(*read_back.get(((i+j*h)*4 + 1) as uint), 255);
            assert_eq!(*read_back.get(((i+j*h)*4 + 2) as uint), 0);
            assert_eq!(*read_back.get(((i+j*h)*4 + 3) as uint), 255);
        }
    }

    ctx.destroy_render_target(rt);
    ctx.destroy_texture(target_texture);
}

} // mod
