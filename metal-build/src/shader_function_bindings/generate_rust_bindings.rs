use super::{
    generate_metal_ast::generate_metal_ast,
    parse_metal_ast::{parse_shader_functions_from_reader, Binds, Function, FunctionConstant},
};
use std::{
    borrow::Cow,
    io::{Read, Write},
    path::Path,
};

const RUST_KEYWORDS: &[&'static str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

fn escape_name(name: &str) -> Cow<str> {
    if RUST_KEYWORDS.contains(&name) {
        Cow::Owned(format!("r#{name}").to_owned())
    } else {
        Cow::Borrowed(name)
    }
}

pub fn generate_shader_function_bindings<P: AsRef<Path>, W: Write>(shader_file: P, writer: &mut W) {
    generate_metal_ast(shader_file, |stdout| {
        generate_shader_function_bindings_from_reader(stdout, writer)
    });
}

pub fn generate_shader_function_bindings_from_reader<R: Read, W: Write>(
    shader_file_reader: R,
    writer: &mut W,
) {
    let mut w = |s: &str| {
        writer
            .write_all(s.as_bytes())
            .expect("Unable to write shader_bindings.rs file (shader function bindings)");
    };
    let (fn_consts, fns) = parse_shader_functions_from_reader(shader_file_reader);
    w(r#"
/****************
 Shader functions
*****************/
"#);
    for Function {
        fn_name,
        binds,
        shader_type,
        referenced_function_constants,
    } in fns
    {
        use Binds::*;
        let rust_shader_name = escape_name(&fn_name);
        let shader_type_titlecase = shader_type.titlecase();
        let rust_function_binds_name = if binds.is_empty() {
            "NoBinds".to_owned()
        } else {
            w(&format!(
                r#"
#[allow(non_camel_case_types)]
pub struct {fn_name}_binds<'c> {{"#
            ));
            for bind in &binds {
                match bind {
                    Buffer {
                        name,
                        data_type,
                        bind_type,
                        ..
                    } => {
                        let rust_shader_bind_name = escape_name(&name);
                        w(&format!(
                            r#"
    pub {rust_shader_bind_name}: Bind{bind_type}<'c, {data_type}>,"#
                        ));
                    }
                    Texture { name, .. } => {
                        let rust_shader_bind_name = escape_name(&name);
                        w(&format!(
                            r#"
    pub {rust_shader_bind_name}: BindTexture<'c>,"#
                        ));
                    }
                }
            }
            w(&format!(
                r#"
}}
impl FunctionBinds for {fn_name}_binds<'_> {{
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {{"#
            ));
            for bind in &binds {
                match bind {
                    Buffer {
                        name,
                        bind_type,
                        index,
                        ..
                    } => {
                        let bind_type = bind_type.to_string().to_ascii_lowercase();
                        let rust_shader_bind_name = escape_name(name);
                        w(&format!(
                            r#"
        E::encode_{bind_type}(encoder, self.{rust_shader_bind_name}, {index});"#
                        ));
                    }
                    Texture { name, index, .. } => {
                        let rust_shader_bind_name = escape_name(name);
                        w(&format!(
                            r#"
        E::encode_texture(encoder, self.{rust_shader_bind_name}, {index});"#
                        ));
                    }
                }
            }
            w(r#"
    }
}
"#);
            format!("{fn_name}_binds<'c>")
        };
        w(&format!(
            r#"
#[allow(non_camel_case_types)]
pub struct {rust_shader_name}"#
        ));
        if referenced_function_constants.is_empty() {
            w(";");
        } else {
            w(" {");
            for fn_const_ref in &referenced_function_constants {
                let FunctionConstant {
                    name, data_type, ..
                } = &fn_consts[usize::from(fn_const_ref)];
                w(&format!(
                    r#"
    pub {name}: {data_type},"#
                ));
            }
            w(&r#"
}"#);
        }

        w(&format!(
            r#"
impl metal_app::pipeline::function::Function for {rust_shader_name} {{
    const FUNCTION_NAME: &'static str = "{fn_name}";
    type Binds<'c> = {rust_function_binds_name};
    type Type = {shader_type_titlecase}FunctionType;"#
        ));
        if !referenced_function_constants.is_empty() {
            w(r#"
    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {
        let fcv = FunctionConstantValues::new();"#);
            for fn_const_ref in &referenced_function_constants {
                let FunctionConstant {
                    name,
                    data_type,
                    index,
                } = &fn_consts[usize::from(fn_const_ref)];
                w(&format!(
                    r#"
        fcv.set_constant_value_at_index((&self.{name} as *const _) as _, {data_type}::MTL_DATA_TYPE, {index});"#
                ));
            }
            w(r#"
        Some(fcv)
    }"#);
        }
        w(r#"
}
"#);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod generate_shader_function_bindings {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn test() {
            let expected = &format!(
                r#"
/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct test_vertex_binds<
    'a,
    Buf0_T: BindMany<float>,
    Buf1_T: Bind<float2>,
    Buf2_T: BindMany<float3>,
    Buf3_T: Bind<float3>,
    Buf5_T: Bind<TestStruct>,
    Buf4_T: BindMany<TestStruct>,
> {{
    pub buf0: Buf0_T,
    pub buf1: Buf1_T,
    pub buf2: Buf2_T,
    pub buf3: Buf3_T,
    pub tex1: BindTexture<'a>,
    pub buf5: Buf5_T,
    pub buf4: Buf4_T,
}}
struct test_vertex_binder<'a, F: PipelineFunctionType>(&'a F::CommandEncoder);
impl<'a, F: PipelineFunctionType> FunctionBinder<'a, F> for test_vertex_binder<'a, F> {{
    fn new(e: &'a F::CommandEncoder) -> Self {{
        Self(e)
    }}
}}
impl<F: PipelineFunctionType> test_vertex_binder<'_, F> {{
    fn bind<
        'a,
        Buf0_T: BindMany<float>,
        Buf1_T: Bind<float2>,
        Buf2_T: BindMany<float3>,
        Buf3_T: Bind<float3>,
        Buf5_T: Bind<TestStruct>,
        Buf4_T: BindMany<TestStruct>,
    >(
        &self,
        binds: test_vertex_binds<
            Buf0_T,
            Buf1_T,
            Buf2_T,
            Buf3_T,
            Buf5_T,
            Buf4_T
        >,
    ) {{
        binds.buf0.bind::<F>(self.0, 0);
        binds.buf1.bind::<F>(self.0, 1);
        binds.buf2.bind::<F>(self.0, 2);
        binds.buf3.bind::<F>(self.0, 3);
        binds.tex1.bind::<F>(self.0, 1);
        binds.buf5.bind::<F>(self.0, 5);
        binds.buf4.bind::<F>(self.0, 4);
    }}
}}

#[allow(non_camel_case_types)]
pub struct test_vertex {{
    pub A_Bool: bool,
}}
impl metal_app::pipeline::function::Function for test_vertex {{
    const FUNCTION_NAME: &'static str = "test_vertex";
    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {{
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index((&self.A_Bool as *const _) as _, bool::MTL_DATA_TYPE, 0);
        Some(fcv)
    }}
}}
impl PipelineFunction<VertexFunctionType> for test_vertex {{
    type Binder<'a> = test_vertex_binder<'a>;
}}

#[allow(non_camel_case_types)]
pub struct test_fragment_binds<
    'a,
    Buf0_T: BindMany<float>,
    Buf1_T: Bind<float2>,
    Buf2_T: BindMany<float3>,
    Buf3_T: Bind<float3>,
    Buf5_T: Bind<TestStruct>,
    Buf4_T: BindMany<TestStruct>,
> {{
    pub buf0: Buf0_T,
    pub buf1: Buf1_T,
    pub buf2: Buf2_T,
    pub buf3: Buf3_T,
    pub tex1: BindTexture<'a>,
    pub buf5: Buf5_T,
    pub buf4: Buf4_T,
}}
struct test_fragment_binder<'a, F: PipelineFunctionType>(&'a F::CommandEncoder);
impl<'a, F: PipelineFunctionType> FunctionBinder<'a, F> for test_fragment_binder<'a, F> {{
    fn new(e: &'a F::CommandEncoder) -> Self {{
        Self(e)
    }}
}}
impl<F: PipelineFunctionType> test_fragment_binder<'_, F> {{
    fn bind<
        'a,
        Buf0_T: BindMany<float>,
        Buf1_T: Bind<float2>,
        Buf2_T: BindMany<float3>,
        Buf3_T: Bind<float3>,
        Buf5_T: Bind<TestStruct>,
        Buf4_T: BindMany<TestStruct>,
    >(
        &self,
        binds: test_vertex_binds<
            Buf0_T,
            Buf1_T,
            Buf2_T,
            Buf3_T,
            Buf5_T,
            Buf4_T
        >,
    ) {{
        binds.buf0.bind::<F>(self.0, 0);
        binds.buf1.bind::<F>(self.0, 1);
        binds.buf2.bind::<F>(self.0, 2);
        binds.buf3.bind::<F>(self.0, 3);
        binds.tex1.bind::<F>(self.0, 1);
        binds.buf5.bind::<F>(self.0, 5);
        binds.buf4.bind::<F>(self.0, 4);
    }}
}}

#[allow(non_camel_case_types)]
pub struct test_fragment {{
    pub A_Float: float,
    pub A_Uint: uint,
}}
impl metal_app::pipeline::function::Function for test_fragment {{
    const FUNCTION_NAME: &'static str = "test_fragment";
    #[inline]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {{
        let fcv = FunctionConstantValues::new();
        fcv.set_constant_value_at_index((&self.A_Float as *const _) as _, float::MTL_DATA_TYPE, 1);
        fcv.set_constant_value_at_index((&self.A_Uint as *const _) as _, uint::MTL_DATA_TYPE, 3);
        Some(fcv)
    }}
}}
impl PipelineFunction<VertexFunctionType> for test_fragment {{
    type Binder<'a> = test_fragment_binder<'a>;
}}
"#
            );
            let shader_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("test_shader_src")
                .join("shader_fn")
                .canonicalize()
                .expect("Failed to canonicalize path to test_shader_src/deps directory");
            let shader_file = shader_dir.join("shaders.metal");
            let mut actual = Vec::<u8>::new();
            generate_shader_function_bindings(shader_file, &mut actual);
            let actual = unsafe { std::str::from_utf8_unchecked(&actual) };

            pretty_assertions::assert_eq!(actual, expected);
        }
    }

    mod generate_shader_function_bindings_from_reader {
        use super::*;
        use crate::shader_function_bindings::parse_metal_ast::BindType;

        fn test(input: &[u8], expected: &str) {
            let mut actual = Vec::<u8>::new();
            generate_shader_function_bindings_from_reader(input, &mut actual);
            let actual = unsafe { std::str::from_utf8_unchecked(&actual) };
            pretty_assertions::assert_eq!(actual, expected);
        }

        #[test]
        fn test_no_binds() {
            test(
                format!("\
TranslationUnitDecl 0x14d8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14d874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14d830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14d874928 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x13d87ef50 <line:3:1, col:17> col:17 Namespace 0x14d8749f0 'metal'
|-FunctionDecl 0x13da41288 <line:12:1, line:14:15> line:12:8 test 'float4 ()'
| |-CompoundStmt 0x13da413f0 <line:14:3, col:15>
| | `-ReturnStmt 0x13da413d8 <col:5, col:12>
| |   `-ImplicitCastExpr 0x13da413c0 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13da413a8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x13da41388 <col:12> 'int' 0
| `-MetalVertexAttr 0x13da41330 <line:11:3>
`-<undeserialized declarations>
").as_bytes(),
                    r#"
/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct test;
impl metal_app::pipeline::function::Function for test {
    const FUNCTION_NAME: &'static str = "test";
    type Binds<'c> = NoBinds;
    type Type = VertexFunctionType;
}
"#
            );
        }

        #[test]
        fn test_bind_buffer() {
            struct Setup {
                fn_name: &'static str,
                bind_name: &'static str,
                multiplicity: &'static str,
                address_space: &'static str,
                data_type: &'static str,
                bind_index: u8,
                bind_type: BindType,
                // TODO: Implement buffer immutability (use const generic, like buffer index)
                #[allow(dead_code)]
                immutable: bool,
            }
            for Setup {
                fn_name,
                bind_name,
                multiplicity,
                address_space,
                data_type,
                bind_index,
                bind_type,
                immutable: _,
            } in [
                Setup {
                    fn_name: "test1",
                    bind_name: "buf_a",
                    multiplicity: "*",
                    address_space: "device",
                    data_type: "uint",
                    bind_index: 0,
                    bind_type: BindType::Many,
                    immutable: false,
                },
                Setup {
                    fn_name: "test2",
                    bind_name: "buf_b",
                    multiplicity: "&",
                    address_space: "device",
                    data_type: "TestStruct",
                    bind_index: 1,
                    bind_type: BindType::One,
                    immutable: false,
                },
                Setup {
                    fn_name: "test3",
                    bind_name: "buf_c",
                    multiplicity: "*",
                    address_space: "const constant",
                    data_type: "float4",
                    bind_index: 2,
                    bind_type: BindType::Many,
                    immutable: true,
                },
                Setup {
                    fn_name: "test4",
                    bind_name: "buf_d",
                    multiplicity: "&",
                    address_space: "const constant",
                    data_type: "float4x4",
                    bind_index: 3,
                    bind_type: BindType::One,
                    immutable: true,
                },
                Setup {
                    fn_name: RUST_KEYWORDS[0],
                    bind_name: "in",
                    multiplicity: "&",
                    address_space: "const constant",
                    data_type: "float4x4",
                    bind_index: 3,
                    bind_type: BindType::One,
                    immutable: true,
                },
            ] {
                test(
                    format!("\
TranslationUnitDecl 0x14d8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14d874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14d830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14d874928 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x13d87ef50 <line:3:1, col:17> col:17 Namespace 0x14d8749f0 'metal'
|-FunctionDecl 0x13da41288 <line:12:1, line:14:15> line:12:8 {fn_name} 'float4 ({address_space} metal::{data_type} {multiplicity})'
| |-ParmVarDecl 0x13d88d0c8 <line:13:5, col:24> col:24 {bind_name} '{address_space} metal::{data_type} {multiplicity}'
| | `-MetalBufferIndexAttr 0x13d88d128 <col:31, col:39>
| |   `-IntegerLiteral 0x13d88d000 <col:38> 'int' {bind_index}
| |-CompoundStmt 0x13da413f0 <line:14:3, col:15>
| | `-ReturnStmt 0x13da413d8 <col:5, col:12>
| |   `-ImplicitCastExpr 0x13da413c0 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13da413a8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x13da41388 <col:12> 'int' 0
| `-MetalVertexAttr 0x13da41330 <line:11:3>
`-<undeserialized declarations>
").as_bytes(),
                    {
                        let rust_shader_name = escape_name(fn_name);
                        let rust_shader_bind_name = escape_name(bind_name);
                        let bind_type_lowercase = bind_type.to_string().to_ascii_lowercase();
                        &format!(r#"
/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct {fn_name}_binds<'c> {{
    pub {rust_shader_bind_name}: Bind{bind_type}<'c, {data_type}>,
}}
impl FunctionBinds for {fn_name}_binds<'_> {{
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {{
        E::encode_{bind_type_lowercase}(encoder, self.{rust_shader_bind_name}, {bind_index});
    }}
}}

#[allow(non_camel_case_types)]
pub struct {rust_shader_name};
impl metal_app::pipeline::function::Function for {rust_shader_name} {{
    const FUNCTION_NAME: &'static str = "{fn_name}";
    type Binds<'c> = {fn_name}_binds<'c>;
    type Type = VertexFunctionType;
}}
"#)
                        }
                );
            }
        }

        #[test]
        fn test_bind_texture() {
            let fn_name = "test7";
            let bind_name = "buf_e";
            let bind_index = 5;
            test(
            format!("\
TranslationUnitDecl 0x1268302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x126874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x126830f20 '__metal_intersection_query_t'
|-ImportDecl 0x1268748f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x116860950 <line:3:1, col:17> col:17 Namespace 0x1268749f0 'metal'
|-FunctionDecl 0x116879ef8 <line:6:1, line:8:15> line:6:8 {fn_name} 'float4 (texture2d<half>)'
| |-ParmVarDecl 0x116879d78 <line:7:5, col:21> col:21 {bind_name} 'texture2d<half>':'metal::texture2d<half, metal::access::sample, void>'
| | `-MetalTextureIndexAttr 0x116879dd8 <col:28, col:37>
| |   `-IntegerLiteral 0x116879d28 <col:36> 'int' {bind_index}
| |-CompoundStmt 0x116995340 <line:8:3, col:15>
| | `-ReturnStmt 0x116995328 <col:5, col:12>
| |   `-ImplicitCastExpr 0x116995310 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x1169952f8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x1169952d8 <col:12> 'int' 0
| `-MetalFragmentAttr 0x116879fa0 <line:5:3>
`-<undeserialized declarations>
").as_bytes(),
            &format!(r#"
/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct {fn_name}_binds<'c> {{
    pub {bind_name}: BindTexture<'c>,
}}
impl FunctionBinds for {fn_name}_binds<'_> {{
    #[inline]
    fn encode_binds<E: BindEncoder>(self, encoder: &RenderCommandEncoderRef) {{
        E::encode_texture(encoder, self.{bind_name}, {bind_index});
    }}
}}

#[allow(non_camel_case_types)]
pub struct {fn_name};
impl metal_app::pipeline::function::Function for {fn_name} {{
    const FUNCTION_NAME: &'static str = "{fn_name}";
    type Binds<'c> = {fn_name}_binds<'c>;
    type Type = FragmentFunctionType;
}}
"#),
            );
        }
    }
}
