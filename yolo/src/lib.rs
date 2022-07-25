#![feature(generic_associated_types)]
#![feature(portable_simd)]
use metal_app::{
    metal::*,
    metal_types::*,
    pipeline::{bind::*, function, pipeline_function::*, render_pipeline::*},
    typed_buffer::*,
    DEFAULT_RESOURCE_OPTIONS,
};
use std::simd::f32x4;

// Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
struct Vertex1Binds<V: BindMany<f32>> {
    v_bind1: V,
    v_bind2: V,
    v_bind3: V,
    v_bind4: V,
}
struct Vertex1Binder<'a>(&'a RenderCommandEncoderRef);
impl<'a> FunctionBinder<'a, VertexFunctionType> for Vertex1Binder<'a> {
    #[inline(always)]
    fn new(e: &'a RenderCommandEncoderRef) -> Self {
        Self(e)
    }
}
impl Vertex1Binder<'_> {
    #[inline]
    fn bind<V: BindMany<f32>>(&self, binds: Vertex1Binds<V>) {
        binds.v_bind1.bind::<VertexFunctionType>(self.0, 0);
        binds.v_bind2.bind::<VertexFunctionType>(self.0, 1);
        binds.v_bind3.bind::<VertexFunctionType>(self.0, 2);
        binds.v_bind4.bind::<VertexFunctionType>(self.0, 4);
    }
}

struct Vertex1 {
    function_constant_1: bool,
}
impl function::Function for Vertex1 {
    const FUNCTION_NAME: &'static str = "vertex1";
    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index(
            (&self.function_constant_1 as *const _) as _,
            bool::MTL_DATA_TYPE,
            1,
        );
        Some(fcv)
    }
}
impl PipelineFunction<VertexFunctionType> for Vertex1 {
    type Binder<'a> = Vertex1Binder<'a>;
}

// Assumed to be generated by metal-build (shader_function_parser/generate_shader_binds)
struct Frag1Binds<'a, F1: Bind<float4>> {
    f_bind1: F1,
    f_tex2: BindTexture<'a>,
}
struct Fragment1Binder<'a>(&'a RenderCommandEncoderRef);
impl<'a> FunctionBinder<'a, FragmentFunctionType> for Fragment1Binder<'a> {
    #[inline(always)]
    fn new(e: &'a RenderCommandEncoderRef) -> Self {
        Self(e)
    }
}
impl Fragment1Binder<'_> {
    #[inline]
    fn bind<F1: Bind<float4>>(&self, binds: Frag1Binds<F1>) {
        binds.f_bind1.bind::<FragmentFunctionType>(self.0, 0);
        binds.f_tex2.bind::<FragmentFunctionType>(self.0, 1);
    }
}
struct Fragment1 {
    function_constant_2: bool,
}
impl function::Function for Fragment1 {
    const FUNCTION_NAME: &'static str = "fragment1";

    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index(
            (&self.function_constant_2 as *const _) as _,
            bool::MTL_DATA_TYPE,
            1,
        );
        Some(fcv)
    }
}
impl PipelineFunction<FragmentFunctionType> for Fragment1 {
    type Binder<'a> = Fragment1Binder<'a>;
}

pub fn run() {
    let device = Device::system_default().expect("Failed to get Metal Device");
    let lib = device.new_default_library();
    let command_queue = device.new_command_queue();
    let command_buffer = command_queue.new_command_buffer();

    let texture = device.new_texture(&TextureDescriptor::new());
    let color1 = &texture;

    let f32_buffer = TypedBuffer::<f32>::with_capacity(
        "f32_buffer",
        &device as &DeviceRef,
        1,
        DEFAULT_RESOURCE_OPTIONS,
    );
    let float4_buffer = TypedBuffer::<float4>::with_capacity(
        "float4_buffer",
        &device as &DeviceRef,
        1,
        DEFAULT_RESOURCE_OPTIONS,
    );

    let p: RenderPipeline<1, Vertex1, Fragment1, NoDepth, NoStencil> = RenderPipeline::new(
        "Test",
        &device,
        &lib,
        [(MTLPixelFormat::BGRA8Unorm, BlendMode::NoBlend)],
        Vertex1 {
            function_constant_1: true,
        },
        Fragment1 {
            function_constant_2: true,
        },
        NoDepth,
        NoStencil,
    );
    let pass = p.new_pass(
        "test label",
        command_buffer,
        [(
            color1,
            (0., 0., 0., 0.),
            MTLLoadAction::Clear,
            MTLStoreAction::Store,
        )],
        NoDepth,
        NoStencil,
    );
    pass.vertex_fn.bind(Vertex1Binds {
        v_bind1: BindBytesMany(&[0.]),
        v_bind2: BindBytesMany(&[1.]),
        v_bind3: BindBytesMany(&[2.]),
        v_bind4: BindBytesMany(&[3.]),
    });
    pass.fragment_fn.bind(Frag1Binds {
        f_bind1: BindBytes(&f32x4::from_array([1.; 4]).into()),
        f_tex2: BindTexture(&texture),
    });
    pass.vertex_fn.bind(Vertex1Binds {
        v_bind1: BindBufferAndOffset(&f32_buffer, 0),
        v_bind2: BindBufferAndOffset(&f32_buffer, 1),
        v_bind3: BindBufferAndOffset(&f32_buffer, 2),
        v_bind4: BindBufferAndOffset(&f32_buffer, 3),
    });
    pass.fragment_fn.bind(Frag1Binds {
        f_bind1: BindBufferAndOffset(&float4_buffer, 0),
        f_tex2: BindTexture(&texture),
    });
}
