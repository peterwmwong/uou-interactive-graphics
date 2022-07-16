use std::{
    assert_matches::debug_assert_matches,
    fmt::Display,
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Command, Stdio},
};

use regex::{Captures, Regex};

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum BindType {
    One,
    Many,
}

impl Display for BindType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(
            match self {
                BindType::One => "One",
                BindType::Many => "Many",
            },
            f,
        )
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub(crate) enum ShaderFunctionBind {
    Buffer {
        index: u8,
        name: String,
        data_type: String,
        bind_type: BindType,
        immutable: bool,
    },
    Texture {
        index: u8,
        name: String,
    },
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum ShaderType {
    Vertex,
    Fragment,
}

impl ShaderType {
    pub const fn lowercase(&self) -> &'static str {
        match self {
            ShaderType::Vertex => "vertex",
            ShaderType::Fragment => "fragment",
        }
    }
    pub const fn titlecase(&self) -> &'static str {
        match self {
            ShaderType::Vertex => "Vertex",
            ShaderType::Fragment => "Fragment",
        }
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ShaderFunction {
    pub(crate) fn_name: String,
    pub(crate) binds: Vec<ShaderFunctionBind>,
    // pub(crate) bind_textures: Vec<ShaderFunctionBind>,
    pub(crate) shader_type: ShaderType,
}

impl ShaderFunction {
    fn new(fn_name: String) -> Self {
        Self {
            fn_name,
            binds: vec![],
            shader_type: ShaderType::Vertex,
        }
    }
}

pub fn parse_shader_functions<P: AsRef<Path>>(shader_file: P) -> Vec<ShaderFunction> {
    let mut cmd = Command::new("xcrun")
        .args(&[
            "-sdk",
            "macosx",
            "metal",
            "-std=metal3.0",
            &shader_file.as_ref().to_string_lossy(),
            "-Xclang",
            "-ast-dump",
            "-fsyntax-only",
            "-fno-color-diagnostics",
        ])
        .env_clear()
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn metal command");
    let stdout = cmd
        .stdout
        .as_mut()
        .expect("Failed to access metal command output");
    let shader_fns = parse_shader_functions_from_reader(stdout);
    cmd.wait().unwrap();
    shader_fns
}

/*
Parses shader function information (shader type, function name, arguments, etc.) from Metal (Clang)
AST.

This function's input is expected to be the Metal (Clang) output when running something like...

    > xcrun metal my_shaders.metal -Xclang -ast-dump -fsyntax-only -fno-color-diagnostics

... output that looks like...

    TranslationUnitDecl 0x1438302e8 <<invalid sloc>> <invalid sloc>
    |-TypedefDecl 0x143874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
    | `-BuiltinType 0x143830f20 '__metal_intersection_query_t'
    |-ImportDecl 0x1438748f0 <<built-in>:1:1> col:1 implicit metal_types
    |-UsingDirectiveDecl 0x143931f50 <line:3:1, col:17> col:17 Namespace 0x1438749f0 'metal'
    |-FunctionDecl 0x14a1327a8 <line:9:1, line:11:15> line:9:8 main_vertex 'float4 (const constant packed_float4 *)'
    | |-ParmVarDecl 0x14a132638 <line:10:5, col:29> col:29 buf0 'const constant packed_float4 *'
    | | `-MetalBufferIndexAttr 0x14a132698 <col:36, col:44>
    | |   `-IntegerLiteral 0x14a132570 <col:43> 'int' 0
    | |-CompoundStmt 0x14a132910 <line:11:3, col:15>
    | | `-ReturnStmt 0x14a1328f8 <col:5, col:12>
    | |   `-ImplicitCastExpr 0x14a1328e0 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
    | |     `-ImplicitCastExpr 0x14a1328c8 <col:12> 'float' <IntegralToFloating>
    | |       `-IntegerLiteral 0x14a1328a8 <col:12> 'int' 0
    | `-MetalVertexAttr 0x14a132850 <line:8:3>

... this function returns...

vec![
    ShaderFunction {
        fn_name: "main_vertex".to_owned(),
        binds: vec![
            ShaderFunctionBind::Buffer {
                index: 0,
                name: "buf0".to_owned(),
                data_type: "packed_float4".to_owned(),
                bind_type: BindType::Many,
                immutable: true
            }
        ],
        shader_type: ShaderType::Vertex,
    }
]

*/
pub fn parse_shader_functions_from_reader<R: Read>(shader_file_reader: R) -> Vec<ShaderFunction> {
    // Example: |-FunctionDecl 0x14a1327a8 <line:9:1, line:11:15> line:9:8 main_vertex 'float4 (const constant packed_float4 *)'
    let rx_fn = Regex::new(
        r"^\|-FunctionDecl 0x\w+ <(line|col)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+ (?P<fn_name>\w+) ",
    )
    .unwrap();

    // Example: | |-ParmVarDecl 0x14a132638 <line:10:5, col:29> col:29 yolo 'const constant packed_float4 *'
    let rx_fn_param = Regex::new(r"^\| (?P<last_child>[`|])-ParmVarDecl 0x\w+ <(line|col)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+(?P<invalid> invalid)? (?P<name>\w+) '(?P<address_space>const constant|device) (metal::)?(?P<data_type>[\w]+) (?P<multiplicity>[*&])'").unwrap();

    // Example: | | `-MetalBufferIndexAttr 0x14a132698 <col:36, col:44>
    let rx_fn_param_metal_buffer_index_attr =
        Regex::new(r"^\| \| (?P<last_child>[`|])-MetalBufferIndexAttr ").unwrap();

    // Example: | |   `-IntegerLiteral 0x14a132570 <col:43> 'int' 0
    let rx_fn_param_metal_buffer_index_attr_value = Regex::new(
        r"^\| \|   (?P<last_child>[`|])-IntegerLiteral 0x\w+ <(line|col)(:\d+)+> 'int' (?P<index>\d+)",
    )
    .unwrap();

    // Example: | `-MetalVertexAttr 0x14a132850 <line:8:3>
    // Example: | `-MetalFragmentAttr 0x14a132850 <line:8:3>
    let rx_fn_metal_shader_type_attr =
        Regex::new(r"^\| (?P<last_child>[`|])-Metal(?P<shader_type>Vertex|Fragment)Attr ").unwrap();

    // Example: | `-
    let rx_fn_last_child = Regex::new(r"^\| `-").unwrap();

    enum FunctionChild {
        Last,
        NotLast,
    }
    impl FunctionChild {
        fn is_last_child(c: &Captures<'_>) -> bool {
            const LAST_CHILD_STR: &'static str = "`";
            &c["last_child"] == LAST_CHILD_STR
        }
        fn parse(c: &Captures<'_>) -> Self {
            if Self::is_last_child(c) {
                FunctionChild::Last
            } else {
                FunctionChild::NotLast
            }
        }
    }
    enum State {
        FindingFunctionDeclaration,
        Function(ShaderFunction),
        Param(ShaderFunction, ShaderFunctionBind, FunctionChild),
        ParamBuffer(ShaderFunction, ShaderFunctionBind, FunctionChild),
    }
    let mut shader_fns = vec![];
    let mut parse_next_state = |state: State, l: String| {
        match state {
            State::FindingFunctionDeclaration => {
                if let Some(c) = rx_fn.captures(&l) {
                    return State::Function(ShaderFunction::new(c["fn_name"].to_owned()));
                }
            }
            State::Function(mut fun) => {
                if let Some(c) = rx_fn_param.captures(&l) {
                    // TODO: Implement, maybe this is unecessary if we run shader compilation **before**.
                    // let invalid = &c["invalid"];
                    // TODO: Implement, this should determine immutability of buffers (render pipeline creation).
                    let address_space = &c["address_space"];
                    let name = &c["name"];
                    let data_type = &c["data_type"];
                    let multiplicity = &c["multiplicity"];
                    return State::Param(
                        fun,
                        ShaderFunctionBind::Buffer {
                            index: u8::MAX,
                            name: name.to_owned(),
                            data_type: data_type.to_owned(),
                            bind_type: if multiplicity == "*" {
                                BindType::Many
                            } else {
                                debug_assert_eq!(
                                    multiplicity, "&",
                                    "Unexpected multiplicity, expected '&' or '*'. Line: {l}"
                                );
                                BindType::One
                            },
                            immutable: if address_space == "const constant" {
                                true
                            } else {
                                debug_assert_eq!(
                                    address_space, "device",
                                    "Unexpected multiplicity, expected '&' or '*'. Line: {l}"
                                );
                                false
                            },
                        },
                        FunctionChild::parse(&c),
                    );
                } else if let Some(c) = rx_fn_metal_shader_type_attr.captures(&l) {
                    let shader_type = &c["shader_type"];
                    match shader_type {
                        "Vertex" => fun.shader_type = ShaderType::Vertex,
                        "Fragment" => fun.shader_type = ShaderType::Fragment,
                        _ => panic!("Unexpected Metal function attribute ({shader_type})"),
                    }
                    // TODO: This may not be true for compute or object functions with parameterized attributes
                    // Example: [[object, max_total_threadgroups_per_mesh_grid(kMeshThreadgroups)]]
                    if FunctionChild::is_last_child(&c) {
                        shader_fns.push(fun);
                        return State::FindingFunctionDeclaration;
                    }
                } else if rx_fn_last_child.is_match(&l) {
                    return State::FindingFunctionDeclaration;
                }
                return State::Function(fun);
            }
            State::Param(fun, bind, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_index_attr.captures(&l) {
                    debug_assert!(
                        FunctionChild::is_last_child(&c),
                        "Unsupported: Multiple function param attributes."
                    );
                    return State::ParamBuffer(fun, bind, fun_last_child);
                }
                panic!("Unexpected function param information ({l})");
            }
            State::ParamBuffer(mut fun, bind, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_index_attr_value.captures(&l) {
                    debug_assert!(FunctionChild::is_last_child(&c), "Unexpected function param buffer attribute information to follow (why is this not the last child?)");
                    debug_assert_matches!(bind, ShaderFunctionBind::Buffer { .. }, "Unexpected bind ({bind:?}). State::ParamBuffer expects bind to be ShaderFunctionBind::Buffer?");

                    let index = c["index"].parse::<u8>().expect(&format!(
                        "Failed to parse function param buffer attribute index value ({l})"
                    ));
                    if let ShaderFunctionBind::Buffer {
                        name,
                        data_type,
                        bind_type,
                        immutable,
                        ..
                    } = bind
                    {
                        fun.binds.push(ShaderFunctionBind::Buffer {
                            index,
                            name,
                            data_type,
                            bind_type,
                            immutable,
                        });
                    }
                    match fun_last_child {
                        FunctionChild::Last => {
                            shader_fns.push(fun);
                            return State::FindingFunctionDeclaration;
                        }
                        FunctionChild::NotLast => return State::Function(fun),
                    }
                }
                panic!("Unexpected function param buffer attribute information ({l})");
            }
        }
        state
    };

    let mut state = State::FindingFunctionDeclaration;
    let mut lines = BufReader::new(shader_file_reader).lines();
    while let Some(Ok(line)) = lines.next() {
        state = parse_next_state(state, line);
    }

    shader_fns
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    mod test_parse_shader_functions_from_reader {
        use super::*;

        fn test<const N: usize>(input: &[u8], expected_fns: [ShaderFunction; N]) {
            let expected = Vec::from(expected_fns);
            let actual = parse_shader_functions_from_reader(input);

            assert_eq!(actual, expected);
        }

        #[test]
        fn test_no_binds() {
            for (metal_attr, expected_shader_type) in [
                ("MetalVertexAttr", ShaderType::Vertex),
                ("MetalFragmentAttr", ShaderType::Fragment),
            ] {
                test(format!("\
TranslationUnitDecl 0x11f0302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x12802c460 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x11f030f20 '__metal_intersection_query_t'
|-ImportDecl 0x12802c4f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x10f066d50 <line:3:1, col:17> col:17 Namespace 0x12802c5f0 'metal'
|-FunctionDecl 0x10f067028 <line:6:1, col:27> col:8 test 'float4 ()'
| |-CompoundStmt 0x10f067188 <col:15, col:27>
| | `-ReturnStmt 0x10f067170 <col:17, col:24>
| |   `-ImplicitCastExpr 0x10f067158 <col:24> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x10f067140 <col:24> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x10f067120 <col:24> 'int' 0
| `-{metal_attr} 0x10f0670c8 <line:5:3>
`-<undeserialized declarations>
").as_bytes(),
                    [ShaderFunction {
                        fn_name: "test".to_owned(),
                        binds: vec![],
                        shader_type: expected_shader_type,
                    }]
                );
            }
        }

        #[test]
        fn test_bind_buffer() {
            for (buffer_bind_multiplicity, address_space, expected_bind_type, expected_immutable) in [
                ("*", "device", BindType::Many, false),
                ("&", "device", BindType::One, false),
                ("*", "const constant", BindType::Many, true),
                ("&", "const constant", BindType::One, true),
            ] {
                test(
                    format!("\
TranslationUnitDecl 0x14d8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14d874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14d830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14d874928 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x13d87ef50 <line:3:1, col:17> col:17 Namespace 0x14d8749f0 'metal'
|-FunctionDecl 0x13da41288 <line:12:1, line:14:15> line:12:8 test 'float4 ({address_space} metal::float4x4 {buffer_bind_multiplicity})'
| |-ParmVarDecl 0x13d88d0c8 <line:13:5, col:24> col:24 buf0 '{address_space} metal::float4x4 {buffer_bind_multiplicity}'
| | `-MetalBufferIndexAttr 0x13d88d128 <col:31, col:39>
| |   `-IntegerLiteral 0x13d88d000 <col:38> 'int' 0
| |-CompoundStmt 0x13da413f0 <line:14:3, col:15>
| | `-ReturnStmt 0x13da413d8 <col:5, col:12>
| |   `-ImplicitCastExpr 0x13da413c0 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13da413a8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x13da41388 <col:12> 'int' 0
| `-MetalVertexAttr 0x13da41330 <line:11:3>
`-<undeserialized declarations>
").as_bytes(),
                    [ShaderFunction {
                        fn_name: "test".to_owned(),
                        binds: vec![
                            ShaderFunctionBind::Buffer { index: 0, name: "buf0".to_owned(), data_type: "float4x4".to_owned(), bind_type: expected_bind_type, immutable: expected_immutable }
                        ],
                        shader_type: ShaderType::Vertex,
                    }],
                );

                /*
                struct MyStruct {
                    float one;
                };

                [[vertex]]
                float4 test(
                    constant float  * buf0 [[buffer(0)]],
                    constant float2 & buf1 [[buffer(1)]],
                    device   float3 * buf2 [[buffer(2)]],
                    device   float3 & buf3 [[buffer(3)]],
                    constant MyStruct & buf5 [[buffer(5)]],
                    constant MyStruct * buf4 [[buffer(4)]]
                ) { return 0; }
                 */
                test(
                    b"\
TranslationUnitDecl 0x13a0302e8 <<invalid sloc>> <invalid sloc>
|-ImportDecl 0x13a074928 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x13a839f50 <line:3:1, col:17> col:17 Namespace 0x13a0749f0 'metal'
| `-FieldDecl 0x13a83a1a0 <line:12:5, col:11> col:11 one 'float'
|-FunctionDecl 0x13a854758 <line:16:1, line:23:15> line:16:8 test 'float4 (const constant float *, const constant float2 &, device float3 *, device float3 &, const constant MyStruct &, const constant MyStruct *)'
| |-ParmVarDecl 0x13a83a470 <line:17:5, col:23> col:23 buf0 'const constant float *'
| | `-MetalBufferIndexAttr 0x13a83a4d0 <col:30, col:38>
| |   `-IntegerLiteral 0x13a83a3f0 <col:37> 'int' 0
| |-ParmVarDecl 0x13a83a7a8 <line:18:5, col:23> col:23 buf1 'const constant float2 &'
| | `-MetalBufferIndexAttr 0x13a83a808 <col:30, col:38>
| |   `-IntegerLiteral 0x13a83a6e0 <col:37> 'int' 1
| |-ParmVarDecl 0x13a8540e8 <line:19:5, col:23> col:23 buf2 'device float3 *'
| | `-MetalBufferIndexAttr 0x13a854148 <col:30, col:38>
| |   `-IntegerLiteral 0x13a854020 <col:37> 'int' 2
| |-ParmVarDecl 0x13a854218 <line:20:5, col:23> col:23 buf3 'device float3 &'
| | `-MetalBufferIndexAttr 0x13a854278 <col:30, col:38>
| |   `-IntegerLiteral 0x13a854190 <col:37> 'int' 3
| |-ParmVarDecl 0x13a854338 <line:21:5, col:25> col:25 buf5 'const constant MyStruct &'
| | `-MetalBufferIndexAttr 0x13a854398 <col:32, col:40>
| |   `-IntegerLiteral 0x13a8542c0 <col:39> 'int' 5
| |-ParmVarDecl 0x13a8544f8 <line:22:5, col:25> col:25 buf4 'const constant MyStruct *'
| | `-MetalBufferIndexAttr 0x13a854558 <col:32, col:40>
| |   `-IntegerLiteral 0x13a854498 <col:39> 'int' 4
| |-CompoundStmt 0x13a8548e8 <line:23:3, col:15>
| | `-ReturnStmt 0x13a8548d0 <col:5, col:12>
| |   `-ImplicitCastExpr 0x13a8548b8 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13a8548a0 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x13a854880 <col:12> 'int' 0
| `-MetalVertexAttr 0x13a854828 <line:15:3>
`-<undeserialized declarations>
",
                    [
                        ShaderFunction {
                            fn_name: "test".to_owned(),
                            binds: vec![
                                ShaderFunctionBind::Buffer { index: 0, name: "buf0".to_owned(), data_type: "float".to_owned(), bind_type: BindType::Many, immutable: true },
                                ShaderFunctionBind::Buffer { index: 1, name: "buf1".to_owned(), data_type: "float2".to_owned(), bind_type: BindType::One, immutable: true },
                                ShaderFunctionBind::Buffer { index: 2, name: "buf2".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::Many, immutable: false },
                                ShaderFunctionBind::Buffer { index: 3, name: "buf3".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::One, immutable: false },
                                ShaderFunctionBind::Buffer { index: 5, name: "buf5".to_owned(), data_type: "MyStruct".to_owned(), bind_type: BindType::One, immutable: true },
                                ShaderFunctionBind::Buffer { index: 4, name: "buf4".to_owned(), data_type: "MyStruct".to_owned(), bind_type: BindType::Many, immutable: true },
                            ],
                            shader_type: ShaderType::Vertex,
                        }
                    ]
                );
            }
        }

        #[test]
        fn test_bind_texture() {
            todo!()
        }

        #[test]
        fn test_non_shader_function() {
            test(
                b"\
TranslationUnitDecl 0x12a8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x12a874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x12a830f20 '__metal_intersection_query_t'
|-ImportDecl 0x12a8748f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x12a931f50 <line:3:1, col:17> col:17 Namespace 0x12a8749f0 'metal'
|-FunctionDecl 0x12a9322c8 <line:24:1, col:35> col:8 test 'float2 (float2)'
| |-ParmVarDecl 0x12a9321a0 <col:13, col:20> col:20 uv 'float2':'float __attribute__((ext_vector_type(2)))'
| `-CompoundStmt 0x12a9323f0 <col:24, col:35>
|   `-ReturnStmt 0x12a9323d8 <col:26, col:33>
|     `-ImplicitCastExpr 0x12a9323c0 <col:33> 'float2':'float __attribute__((ext_vector_type(2)))' <VectorSplat>
|       `-ImplicitCastExpr 0x12a9323a8 <col:33> 'float' <IntegralToFloating>
|         `-IntegerLiteral 0x12a932388 <col:33> 'int' 0
`-<undeserialized declarations>
",
                []
            );
        }

        #[test]
        fn test_shader_and_non_shader_functions() {
            /*
            [[vertex]]   float4 test_vertex(constant int *buf0 [[buffer(0)]]) { return 0; }
                         float2 test(float2 uv) { return 0; }
            [[fragment]] half4  test_fragment(device float3 *buf1 [[buffer(1)]]) { return 0; }
             */
            test(
                b"\
TranslationUnitDecl 0x13d8192e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x13d841e60 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x13d819f20 '__metal_intersection_query_t'
|-ImportDecl 0x13d841f28 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x13d8ff950 <line:3:1, col:17> col:17 Namespace 0x13d841ff0 'metal'
|-FunctionDecl 0x13d8ffd78 <line:26:14, col:79> col:21 test_vertex 'float4 (const constant int *)'
| |-ParmVarDecl 0x13d8ffc10 <col:33, col:47> col:47 buf0 'const constant int *'
| | `-MetalBufferIndexAttr 0x13d8ffc70 <col:54, col:62>
| |   `-IntegerLiteral 0x13d8ffb90 <col:61> 'int' 0
| |-CompoundStmt 0x13d8ffee0 <col:67, col:79>
| | `-ReturnStmt 0x13d8ffec8 <col:69, col:76>
| |   `-ImplicitCastExpr 0x13d8ffeb0 <col:76> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13d8ffe98 <col:76> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x13d8ffe78 <col:76> 'int' 0
| `-MetalVertexAttr 0x13d8ffe20 <col:3>
|-FunctionDecl 0x13d9001f8 <line:27:14, col:49> col:21 test 'float2 (float2)'
| |-ParmVarDecl 0x13d9000d0 <col:26, col:33> col:33 uv 'float2':'float __attribute__((ext_vector_type(2)))'
| `-CompoundStmt 0x13d900320 <col:37, col:49>
|   `-ReturnStmt 0x13d900308 <col:39, col:46>
|     `-ImplicitCastExpr 0x13d9002f0 <col:46> 'float2':'float __attribute__((ext_vector_type(2)))' <VectorSplat>
|       `-ImplicitCastExpr 0x13d9002d8 <col:46> 'float' <IntegralToFloating>
|         `-IntegerLiteral 0x13d9002b8 <col:46> 'int' 0
|-FunctionDecl 0x13d914d18 <line:28:14, col:82> col:21 test_fragment 'half4 (device float3 *)'
| |-ParmVarDecl 0x13d914ba8 <col:35, col:50> col:50 buf1 'device float3 *'
| | `-MetalBufferIndexAttr 0x13d914c08 <col:57, col:65>
| |   `-IntegerLiteral 0x13d914ae0 <col:64> 'int' 1
| |-CompoundStmt 0x13d914e80 <col:70, col:82>
| | `-ReturnStmt 0x13d914e68 <col:72, col:79>
| |   `-ImplicitCastExpr 0x13d914e50 <col:79> 'half4':'half __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x13d914e38 <col:79> 'half' <IntegralToFloating>
| |       `-IntegerLiteral 0x13d914e18 <col:79> 'int' 0
| `-MetalFragmentAttr 0x13d914dc0 <col:3>
`-<undeserialized declarations>
",
                [
                    ShaderFunction {
                        fn_name: "test_vertex".to_owned(),
                        binds: vec![
                            ShaderFunctionBind::Buffer { index: 0, name: "buf0".to_owned(), data_type: "int".to_owned(), bind_type: BindType::Many, immutable: true },
                        ],
                        shader_type: ShaderType::Vertex,
                    },
                    ShaderFunction {
                        fn_name: "test_fragment".to_owned(),
                        binds: vec![
                            ShaderFunctionBind::Buffer { index: 1, name: "buf1".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::Many, immutable: false },
                        ],
                        shader_type: ShaderType::Fragment,
                    },
                ]
            );
        }

        #[test]
        fn test_multiple_bind_attributes() {
            // Example Input Snippets:
            //     [[vertex]]   float4 my_fn(device int *input [[buffer(0), function_constant(hasInputBuffer)]]) { ... }
            //     [[fragment]] half4  my_fn(texture2d<half, access::read_write> tex [[raster_order_group(0), texture(0)]]) { ... }
            todo!();
        }

        #[test]
        fn test_multiple_function_attributes() {
            // Example Input Snippets:
            //     [[object, max_total_threadgroups_per_mesh_grid(kMeshThreadgroups)]] void objectShader(mesh_grid_properties mgp) { ... }
            todo!();
        }
    }

    mod test_parse_shader_functions {
        use super::*;

        #[test]
        fn test() {
            let expected = vec![ShaderFunction {
                fn_name: "v_no_args_position_only_return".to_owned(),
                binds: vec![],
                shader_type: ShaderType::Vertex,
            }];

            let shader_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("test_shader_src")
                .join("shader_fn")
                .canonicalize()
                .expect("Failed to canonicalize path to test_shader_src/deps directory");
            let shader_file = shader_dir.join("shaders.metal");
            let actual = parse_shader_functions(shader_file);

            assert_eq!(actual, expected);
        }

        #[test]
        fn test_shader_with_errors() {
            // TODO: Read from stderr to find out if there are any errors and abort if any.
            // Example Input Snippets:
            //     [[vertex]] [[fragment]] float4 my_fn() {}
            //     [[vertex]] void my_fn() {}
            //     [[vertex]] float4 my_fn(float4 no_address_space_woopsies) {}
            todo!();
        }
    }
}
