#![allow(non_camel_case_types)]

pub type TextureFlags = i32;
pub const REPEAT_S          : TextureFlags = 1 << 0;
pub const REPEAT_T          : TextureFlags = 1 << 1;
pub const REPEAT            : TextureFlags = 1 << (REPEAT_S | REPEAT_T) as usize;
pub const CLAMP_S           : TextureFlags = 1 << 2;
pub const CLAMP_T           : TextureFlags = 1 << 3;
pub const CLAMP             : TextureFlags = 1 << (CLAMP_S | CLAMP_T) as usize;
pub const MIN_FILTER_LINEAR : TextureFlags = 1 << 4;
pub const MAG_FILTER_LINEAR : TextureFlags = 1 << 5;
pub const FILTER_LINEAR     : TextureFlags = MIN_FILTER_LINEAR | MAG_FILTER_LINEAR;
pub const MIN_FILTER_NEAREST: TextureFlags = 1 << 6;
pub const MAG_FILTER_NEAREST: TextureFlags = 1 << 7;
pub const FILTER_NEAREST    : TextureFlags = MIN_FILTER_NEAREST | MAG_FILTER_NEAREST;
pub const FLAGS_DEFAULT     : TextureFlags = CLAMP | FILTER_LINEAR;

pub type GeometryFlags = u32;
pub const TRIANGLES             : GeometryFlags = 1 << 3;
pub const LINES                 : GeometryFlags = 1 << 4;
pub const STRIP                 : GeometryFlags = 1 << 5;
pub const LOOP                  : GeometryFlags = 1 << 5;
pub const TRIANGLE_STRIP        : GeometryFlags = TRIANGLES | STRIP;
pub const LINE_STRIP            : GeometryFlags = LINES | STRIP;
pub const LINE_LOOP             : GeometryFlags = LINES | LOOP;

pub type TargetTypes = u32;
pub const COLOR  : TargetTypes = 1 << 0;
pub const DEPTH  : TargetTypes = 1 << 1;
pub const STENCIL: TargetTypes = 1 << 2;

pub type MapFlags = u8;
pub const READ_MAP      : MapFlags = 1;
pub const WRITE_MAP     : MapFlags = 2;
pub const PERSISTENT_MAP: MapFlags = 3;
pub const COHERENT_MAP  : MapFlags = 4;

pub type ErrorFlags = u8;
pub const IGNORE_ERRORS : ErrorFlags = 0;
pub const LOG_ERRORS    : ErrorFlags = 1;
pub const CRASH_ERRORS  : ErrorFlags = 2;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ShaderType {
    Fragment,
    Vertex,
    Geometry,
    Compute,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Feature {
    FragmentShading,
    VertexShading,
    GeometryShading,
    Compute,
    DepthTexture,
    RenderToTexture,
    MultipleRenderTargets,
    InstancedRendering,
}

pub enum FeatureSupport {
    Supported,
    Fallback,
    Unsupported,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum PixelFormat {
    R8G8B8A8,
    R8G8B8X8,
    B8G8R8A8,
    B8G8R8X8,
    A8,
    A_F32,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum UpdateHint {
    Static,
    Stream,
    Dynamic,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BufferType {
    Vertex,
    Index,
    Uniform,
    DrawIndirect,
    TransformFeedback,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BlendMode {
    None,
    Alpha,
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum ResultCode {
    Ok,
    UnknownError,
    UnknownCommand,
    InvalidArgument,
    OutOfMemory,
    InvalidObjectHandle,
    ShaderCompilationFailed,
    ShaderLinkFailed,
    DeviceLost,
    RtMissingAttachment,
    RtIncompleteAttachment,
    RtUnsupported,
}
