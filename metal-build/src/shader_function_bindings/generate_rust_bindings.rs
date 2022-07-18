use super::{
    generate_metal_ast::generate_metal_ast,
    parse_metal_ast::{parse_shader_functions_from_reader, ShaderFunction, ShaderFunctionBind},
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
    w(r#"
/****************
 Shader functions
*****************/
"#);
    for ShaderFunction {
        fn_name,
        binds,
        shader_type,
    } in parse_shader_functions_from_reader(shader_file_reader)
    {
        use ShaderFunctionBind::*;
        let rust_shader_name = escape_name(&fn_name);
        let rust_binds_generic_lifetime = if binds.is_empty() { "" } else { "<'c>" };
        w(&format!(
            r#"
#[allow(non_camel_case_types)]
pub struct {fn_name}_binds{rust_binds_generic_lifetime} {{"#
        ));
        // TODO: Consider to code generation trait to make this more readable...
        //         binds.each_gen_field(|index, name, data_type, bind_type| {
        //             &format!(
        //                 r#"
        // {name}: Bind{bind_type}<'a, {index}, {data_type}>,"#
        //             )
        //         });
        for bind in &binds {
            match bind {
                Buffer {
                    index,
                    name,
                    data_type,
                    bind_type,
                    // TODO: Implement marking buffers as immutible
                    immutable: _,
                } => {
                    let rust_shader_bind_name = escape_name(&name);
                    w(&format!(
                        r#"
    {rust_shader_bind_name}: Bind{bind_type}<'c, {index}, {data_type}>,"#
                    ));
                }
                Texture { name, index } => {
                    let rust_shader_bind_name = escape_name(&name);
                    w(&format!(
                        r#"
    {rust_shader_bind_name}: BindTexture<'c, {index}>,"#
                    ));
                }
            }
        }
        let shader_type_lowercase = shader_type.lowercase();
        let shader_type_titlecase = shader_type.titlecase();
        w(&format!(
            r#"
}}

impl{rust_binds_generic_lifetime} {shader_type_titlecase}ShaderBinds for {fn_name}_binds{rust_binds_generic_lifetime} {{
    #[inline]
    fn encode_{shader_type_lowercase}_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {{"#
        ));
        // TODO: Consider to code generation trait to make this more readable...
        //         binds.each_gen_encode(|name| {
        //             &format!(
        //                 r#"
        // self.{name}.encode_for_{shader_type_lowercase}(encoder);"#
        //             )
        //         });
        for bind in &binds {
            match bind {
                Buffer { name, .. } | Texture { name, .. } => {
                    let rust_shader_bind_name = escape_name(&name);
                    w(&format!(
                        r#"
        self.{rust_shader_bind_name}.encode_for_{shader_type_lowercase}(encoder);"#
                    ))
                }
            }
        }
        w(&format!(
            r#"
    }}
}}

#[allow(non_camel_case_types)]
struct {rust_shader_name};
impl {shader_type_titlecase}Shader for {rust_shader_name} {{
    type Binds<'c> = {fn_name}_binds{rust_binds_generic_lifetime};

    #[inline]
    fn function_name() -> &'static str {{ "{fn_name}" }}
}}
"#
        ));
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
pub struct test_vertex_binds<'c> {{
    buf0: BindMany<'c, 0, float>,
    buf1: BindOne<'c, 1, float2>,
    buf2: BindMany<'c, 2, float3>,
    buf3: BindOne<'c, 3, float3>,
    tex1: BindTexture<'c, 1>,
    buf5: BindOne<'c, 5, TestStruct>,
    buf4: BindMany<'c, 4, TestStruct>,
}}

impl<'c> VertexShaderBinds for test_vertex_binds<'c> {{
    #[inline]
    fn encode_vertex_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {{
        self.buf0.encode_for_vertex(encoder);
        self.buf1.encode_for_vertex(encoder);
        self.buf2.encode_for_vertex(encoder);
        self.buf3.encode_for_vertex(encoder);
        self.tex1.encode_for_vertex(encoder);
        self.buf5.encode_for_vertex(encoder);
        self.buf4.encode_for_vertex(encoder);
    }}
}}

#[allow(non_camel_case_types)]
struct test_vertex;
impl VertexShader for test_vertex {{
    type Binds<'c> = test_vertex_binds<'c>;

    #[inline]
    fn function_name() -> &'static str {{ "test_vertex" }}
}}

#[allow(non_camel_case_types)]
pub struct test_fragment_binds<'c> {{
    buf0: BindMany<'c, 0, float>,
    buf1: BindOne<'c, 1, float2>,
    buf2: BindMany<'c, 2, float3>,
    buf3: BindOne<'c, 3, float3>,
    tex1: BindTexture<'c, 1>,
    buf5: BindOne<'c, 5, TestStruct>,
    buf4: BindMany<'c, 4, TestStruct>,
}}

impl<'c> FragmentShaderBinds for test_fragment_binds<'c> {{
    #[inline]
    fn encode_fragment_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {{
        self.buf0.encode_for_fragment(encoder);
        self.buf1.encode_for_fragment(encoder);
        self.buf2.encode_for_fragment(encoder);
        self.buf3.encode_for_fragment(encoder);
        self.tex1.encode_for_fragment(encoder);
        self.buf5.encode_for_fragment(encoder);
        self.buf4.encode_for_fragment(encoder);
    }}
}}

#[allow(non_camel_case_types)]
struct test_fragment;
impl FragmentShader for test_fragment {{
    type Binds<'c> = test_fragment_binds<'c>;

    #[inline]
    fn function_name() -> &'static str {{ "test_fragment" }}
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

            assert_eq!(actual, expected);
        }
    }

    mod generate_shader_function_bindings_from_reader {
        use super::*;
        use crate::shader_function_bindings::parse_metal_ast::BindType;

        fn test(input: &[u8], expected: &str) {
            let mut actual = Vec::<u8>::new();
            generate_shader_function_bindings_from_reader(input, &mut actual);
            let actual = unsafe { std::str::from_utf8_unchecked(&actual) };
            assert_eq!(actual, expected);
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
                        &format!(r#"
/****************
 Shader functions
*****************/

#[allow(non_camel_case_types)]
pub struct {fn_name}_binds<'c> {{
    {rust_shader_bind_name}: Bind{bind_type}<'c, {bind_index}, {data_type}>,
}}

impl<'c> VertexShaderBinds for {fn_name}_binds<'c> {{
    #[inline]
    fn encode_vertex_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {{
        self.{rust_shader_bind_name}.encode_for_vertex(encoder);
    }}
}}

#[allow(non_camel_case_types)]
struct {rust_shader_name};
impl VertexShader for {rust_shader_name} {{
    type Binds<'c> = {fn_name}_binds<'c>;

    #[inline]
    fn function_name() -> &'static str {{ "{fn_name}" }}
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
    {bind_name}: BindTexture<'c, {bind_index}>,
}}

impl<'c> FragmentShaderBinds for {fn_name}_binds<'c> {{
    #[inline]
    fn encode_fragment_binds<'a, 'b>(&'a self, encoder: &'b RenderCommandEncoderRef) {{
        self.{bind_name}.encode_for_fragment(encoder);
    }}
}}

#[allow(non_camel_case_types)]
struct {fn_name};
impl FragmentShader for {fn_name} {{
    type Binds<'c> = {fn_name}_binds<'c>;

    #[inline]
    fn function_name() -> &'static str {{ "{fn_name}" }}
}}
"#),
        );
        }
    }
}
