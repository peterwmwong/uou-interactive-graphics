use regex::{Captures, Regex};
use std::{
    fmt::Display,
    io::{BufRead, BufReader, Read},
};

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
pub enum ShaderFunctionBind {
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
impl ShaderFunctionBind {
    pub const INVALID_INDEX: u8 = u8::MAX;
    fn with_new_index(self, index: u8) -> Self {
        use ShaderFunctionBind::*;
        match self {
            Buffer {
                name,
                data_type,
                bind_type,
                immutable,
                ..
            } => Buffer {
                index,
                name,
                data_type,
                bind_type,
                immutable,
            },
            Texture { name, .. } => Texture { index, name },
        }
    }
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
    pub fn_name: String,
    pub binds: Vec<ShaderFunctionBind>,
    pub shader_type: ShaderType,
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

/*
Parses shader function information (shader type, function name, arguments, etc.) from Metal (Clang)
AST.

IMPORTANT: Assumes valid AST! Passing an invalid AST has undefined behavior.
- Assumed to be called **AFTER** shader compilation (Metal AIR/Native), which would have failed if
  your shaders were bad for whatever reason (ex. invalid syntax, invalid buffer index, etc.).

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
        r"^\|-FunctionDecl 0x\w+ <([^:]+)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+ (?P<fn_name>\w+) ",
    )
    .unwrap();

    // Example: | |-ParmVarDecl 0x14a132638 <line:10:5, col:29> col:29 yolo 'const constant packed_float4 *'
    // Example: | |-ParmVarDecl 0x116879d78 <line:7:5, col:21> col:21 tex0 'texture2d<half>':'metal::texture2d<half, metal::access::sample, void>'
    let rx_fn_param = Regex::new(r"^\| (?P<last_child>[`|])-ParmVarDecl 0x\w+ <(line|col)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+( used)? (?P<name>\w+) '(?P<address_space>const constant |device |\w+[\w:<>, ]+':')(metal::)?(?P<data_type>[\w:<>, ]+)(?P<multiplicity> [*&]|)'").unwrap();

    // Example: | | `-MetalBufferIndexAttr 0x14a132698 <col:36, col:44>
    let rx_fn_param_metal_buffer_texture_index_attr = Regex::new(
        r"^\| \| (?P<last_child>[`|])-Metal(?P<buffer_or_texture>Buffer|Texture)IndexAttr ",
    )
    .unwrap();

    // Example: | |   `-IntegerLiteral 0x14a132570 <col:43> 'int' 0
    let rx_fn_param_metal_buffer_texture_index_attr_value = Regex::new(
        r"^\| \|   (?P<last_child>[`|])-IntegerLiteral 0x\w+ <(line|col)(:\d+)+> 'int' (?P<index>\d+)",
    )
    .unwrap();

    // Example: | `-MetalVertexAttr 0x14a132850 <line:8:3>
    // Example: | `-MetalFragmentAttr 0x14a132850 <line:8:3>
    let rx_fn_metal_shader_type_attr =
        Regex::new(r"^\| (?P<last_child>[`|])-Metal(?P<shader_type>Vertex|Fragment)Attr ").unwrap();

    // Example: | `-
    let rx_fn_last_child = Regex::new(r"^\| `-").unwrap();

    // Example: | `-
    // Example: | | `-
    // Example: | | |`-
    let rx_last_child_of_any_level = Regex::new(r"^(\| )+`-").unwrap();

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
    struct ShaderFunctionParamInfo {
        address_space: String,
        name: String,
        multiplicity: String,
        data_type: String,
    }
    enum State {
        FindFunction,
        Function(ShaderFunction),
        Param(ShaderFunction, ShaderFunctionParamInfo, FunctionChild),
        ParamBufferTexture(ShaderFunction, ShaderFunctionBind, FunctionChild),
    }
    let mut shader_fns = vec![];
    let mut parse_next_state = |state: State, l: String| {
        match state {
            State::FindFunction => {
                if let Some(c) = rx_fn.captures(&l) {
                    return State::Function(ShaderFunction::new(c["fn_name"].to_owned()));
                }
            }
            State::Function(mut fun) => {
                if let Some(c) = rx_fn_param.captures(&l) {
                    // TODO: Implement, this should determine immutability of buffers (render pipeline creation).
                    return State::Param(
                        fun,
                        ShaderFunctionParamInfo {
                            address_space: c["address_space"].to_owned(),
                            name: c["name"].to_owned(),
                            multiplicity: c["multiplicity"].to_owned(),
                            data_type: c["data_type"].to_owned(),
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
                        return State::FindFunction;
                    }
                } else if rx_fn_last_child.is_match(&l) {
                    return State::FindFunction;
                }
                return State::Function(fun);
            }
            State::Param(fun, info, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_texture_index_attr.captures(&l) {
                    debug_assert!(
                        FunctionChild::is_last_child(&c),
                        "Unsupported: Multiple function param attributes."
                    );
                    let buffer_or_texture = &c["buffer_or_texture"];
                    return State::ParamBufferTexture(
                        fun,
                        if buffer_or_texture == "Buffer" {
                            ShaderFunctionBind::Buffer {
                                index: ShaderFunctionBind::INVALID_INDEX,
                                name: info.name,
                                data_type: info.data_type,
                                bind_type: if info.multiplicity == " *" {
                                    BindType::Many
                                } else {
                                    debug_assert_eq!(
                                        info.multiplicity, " &",
                                        "Unexpected multiplicity, expected '&' or '*'. Line: {l}"
                                    );
                                    BindType::One
                                },
                                immutable: if info.address_space == "const constant " {
                                    true
                                } else {
                                    debug_assert_eq!(
                                        info.address_space, "device ",
                                        "Unexpected multiplicity, expected '&' or '*'. Line: {l}"
                                    );
                                    false
                                },
                            }
                        } else {
                            debug_assert_eq!(buffer_or_texture, "Texture");
                            ShaderFunctionBind::Texture {
                                index: ShaderFunctionBind::INVALID_INDEX,
                                name: info.name,
                            }
                        },
                        fun_last_child,
                    );
                }
                if rx_last_child_of_any_level.is_match(&l) {
                    return match fun_last_child {
                        FunctionChild::Last => State::FindFunction,
                        FunctionChild::NotLast => State::Function(fun),
                    };
                }
                return State::Param(fun, info, fun_last_child);
            }
            State::ParamBufferTexture(mut fun, bind, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_texture_index_attr_value.captures(&l) {
                    debug_assert!(FunctionChild::is_last_child(&c), "Unexpected function param buffer attribute information to follow (why is this not the last child?)");
                    let index = c["index"].parse::<u8>().expect(&format!(
                        "Failed to parse function param buffer attribute index value ({l})"
                    ));
                    fun.binds.push(bind.with_new_index(index));
                    match fun_last_child {
                        FunctionChild::Last => {
                            shader_fns.push(fun);
                            return State::FindFunction;
                        }
                        FunctionChild::NotLast => return State::Function(fun),
                    }
                }
                panic!("Unexpected function param buffer attribute information ({l})");
            }
        }
        state
    };

    let mut state = State::FindFunction;
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
        fn test_non_binds() {
            /*
            [[vertex]]
            float4 test(uint vertex_id [[vertex_id]]) {
                return 0;
            }
            */
            test(format!("\
TranslationUnitDecl 0x14c8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14c874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14c830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14c874928 <metal-build/test_shader_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-UsingDirectiveDecl 0x14c931f50 <line:3:1, col:17> col:17 Namespace 0x14c8749f0 'metal'
|-FunctionDecl 0x14c932498 <line:6:1, line:8:15> line:6:8 test 'float4 (uint)'
| |-ParmVarDecl 0x14c932338 <line:7:5, col:10> col:10 vertex_id 'uint':'unsigned int'
| | `-MetalVertexIdAttr 0x14c932398 <col:22>
| |-CompoundStmt 0x14c932600 <line:8:3, col:15>
| | `-ReturnStmt 0x14c9325e8 <col:5, col:12>
| |   `-ImplicitCastExpr 0x14c9325d0 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x14c9325b8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x14c932598 <col:12> 'int' 0
| `-MetalVertexAttr 0x14c932540 <line:5:3>
`-<undeserialized declarations>
").as_bytes(),
                [ShaderFunction {
                    fn_name: "test".to_owned(),
                    binds: vec![],
                    shader_type: ShaderType::Vertex,
                }]
            );
        }

        #[test]
        fn test_no_binds() {
            for path in ["line", "proj-2-transformations/shader_src/shaders.metal"] {
                for (metal_attr, expected_shader_type) in [
                    ("MetalVertexAttr", ShaderType::Vertex),
                    ("MetalFragmentAttr", ShaderType::Fragment),
                ] {
                    /*
                    [[vertex]]
                    float4 test() {
                        return 0;
                    }
                    */
                    test(format!("\
TranslationUnitDecl 0x11f0302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x12802c460 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x11f030f20 '__metal_intersection_query_t'
|-ImportDecl 0x12802c4f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x10f066d50 <line:3:1, col:17> col:17 Namespace 0x12802c5f0 'metal'
|-FunctionDecl 0x10f067028 <{path}:6:1, col:27> col:8 test 'float4 ()'
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
        }

        #[test]
        fn test_bind_buffer() {
            for used in [" used", ""] {
                for (
                    buffer_bind_multiplicity,
                    address_space,
                    expected_bind_type,
                    expected_immutable,
                ) in [
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
| |-ParmVarDecl 0x13d88d0c8 <line:13:5, col:24> col:24{used} buf0 '{address_space} metal::float4x4 {buffer_bind_multiplicity}'
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
                }
            }

            /*
            struct TestStruct {
                float one;
            };

            [[vertex]]
            float4 test(
                constant float  *        buf0      [[buffer(0)]],
                constant float2 &        buf1      [[buffer(1)]],
                            uint            vertex_id [[vertex_id]],
                device   float3 *        buf2      [[buffer(2)]],
                device   float3 &        buf3      [[buffer(3)]],
                            texture2d<half> tex1      [[texture(1)]],
                constant TestStruct &    buf5      [[buffer(5)]],
                constant TestStruct *    buf4      [[buffer(4)]]
            ) { return 0; }
            */
            test(
                b"\
TranslationUnitDecl 0x1210302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x121074860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x121030f20 '__metal_intersection_query_t'
|-ImportDecl 0x1210748f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x121131f50 <line:3:1, col:17> col:17 Namespace 0x1210749f0 'metal'
|-FunctionDecl 0x1211513b8 <line:15:1, line:24:15> line:15:8 test 'float4 (const constant float *, const constant float2 &, uint, device float3 *, device float3 &, texture2d<half>, const constant TestStruct &, const constant TestStruct *)'
| |-ParmVarDecl 0x121132470 <line:16:5, col:30> col:30 buf0 'const constant float *'
| | `-MetalBufferIndexAttr 0x1211324d0 <col:42, col:50>
| |   `-IntegerLiteral 0x1211323f0 <col:49> 'int' 0
| |-ParmVarDecl 0x1211327a8 <line:17:5, col:30> col:30 buf1 'const constant float2 &'
| | `-MetalBufferIndexAttr 0x121132808 <col:42, col:50>
| |   `-IntegerLiteral 0x1211326e0 <col:49> 'int' 1
| |-ParmVarDecl 0x12114c000 <line:18:14, col:30> col:30 vertex_id 'uint':'unsigned int'
| | `-MetalVertexIdAttr 0x12114c060 <col:42>
| |-ParmVarDecl 0x12114c338 <line:19:5, col:30> col:30 buf2 'device float3 *'
| | `-MetalBufferIndexAttr 0x12114c398 <col:42, col:50>
| |   `-IntegerLiteral 0x12114c270 <col:49> 'int' 2
| |-ParmVarDecl 0x12114c468 <line:20:5, col:30> col:30 buf3 'device float3 &'
| | `-MetalBufferIndexAttr 0x12114c4c8 <col:42, col:50>
| |   `-IntegerLiteral 0x12114c3e0 <col:49> 'int' 3
| |-ParmVarDecl 0x121150dc8 <line:21:14, col:30> col:30 tex1 'texture2d<half>':'metal::texture2d<half, metal::access::sample, void>'
| | `-MetalTextureIndexAttr 0x121150e28 <col:42, col:51>
| |   `-IntegerLiteral 0x121150d78 <col:50> 'int' 1
| |-ParmVarDecl 0x121150ee8 <line:22:5, col:30> col:30 buf5 'const constant TestStruct &'
| | `-MetalBufferIndexAttr 0x121150f48 <col:42, col:50>
| |   `-IntegerLiteral 0x121150e70 <col:49> 'int' 5
| |-ParmVarDecl 0x1211510f8 <line:23:5, col:30> col:30 buf4 'const constant TestStruct *'
| | `-MetalBufferIndexAttr 0x121151158 <col:42, col:50>
| |   `-IntegerLiteral 0x1211510a0 <col:49> 'int' 4
| |-CompoundStmt 0x121283140 <line:24:3, col:15>
| | `-ReturnStmt 0x121283128 <col:5, col:12>
| |   `-ImplicitCastExpr 0x121283110 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x1212830f8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x1212830d8 <col:12> 'int' 0
| `-MetalVertexAttr 0x121151498 <line:14:3>
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
                            ShaderFunctionBind::Texture { index: 1, name: "tex1".to_owned() },
                            ShaderFunctionBind::Buffer { index: 5, name: "buf5".to_owned(), data_type: "TestStruct".to_owned(), bind_type: BindType::One, immutable: true },
                            ShaderFunctionBind::Buffer { index: 4, name: "buf4".to_owned(), data_type: "TestStruct".to_owned(), bind_type: BindType::Many, immutable: true },
                        ],
                        shader_type: ShaderType::Vertex,
                    }
                ]
            );
        }

        #[test]
        fn test_bind_texture() {
            for used in [" used", ""] {
                test(
                &format!("\
TranslationUnitDecl 0x1268302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x126874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x126830f20 '__metal_intersection_query_t'
|-ImportDecl 0x1268748f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x116860950 <line:3:1, col:17> col:17 Namespace 0x1268749f0 'metal'
|-FunctionDecl 0x116879ef8 <line:6:1, line:8:15> line:6:8 test 'float4 (texture2d<half>)'
| |-ParmVarDecl 0x116879d78 <line:7:5, col:21> col:21{used} tex0 'texture2d<half>':'metal::texture2d<half, metal::access::sample, void>'
| | `-MetalTextureIndexAttr 0x116879dd8 <col:28, col:37>
| |   `-IntegerLiteral 0x116879d28 <col:36> 'int' 0
| |-CompoundStmt 0x116995340 <line:8:3, col:15>
| | `-ReturnStmt 0x116995328 <col:5, col:12>
| |   `-ImplicitCastExpr 0x116995310 <col:12> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x1169952f8 <col:12> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x1169952d8 <col:12> 'int' 0
| `-MetalFragmentAttr 0x116879fa0 <line:5:3>
`-<undeserialized declarations>
").as_bytes(),
                    [
                        ShaderFunction {
                            shader_type: ShaderType::Fragment,
                            fn_name: "test".to_owned(),
                            binds: vec![
                                ShaderFunctionBind::Texture { index: 0, name: "tex0".to_owned() },
                            ],
                        }
                    ]
                );
            }
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
            // TODO: Implement
        }

        #[test]
        fn test_multiple_function_attributes() {
            // Example Input Snippets:
            //     [[object, max_total_threadgroups_per_mesh_grid(kMeshThreadgroups)]] void objectShader(mesh_grid_properties mgp) { ... }
            // TODO: Implement
        }
    }

    mod test_parse_shader_functions {
        use super::*;
        use crate::shader_function_bindings::generate_metal_ast::generate_metal_ast;

        #[test]
        fn test() {
            let expected = vec![
                ShaderFunction {
                    fn_name: "test_vertex".to_owned(),
                    binds: vec![
                        ShaderFunctionBind::Buffer {
                            index: 0,
                            name: "buf0".to_owned(),
                            data_type: "float".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 1,
                            name: "buf1".to_owned(),
                            data_type: "float2".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 2,
                            name: "buf2".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::Many,
                            immutable: false,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 3,
                            name: "buf3".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::One,
                            immutable: false,
                        },
                        ShaderFunctionBind::Texture {
                            index: 1,
                            name: "tex1".to_owned(),
                        },
                        ShaderFunctionBind::Buffer {
                            index: 5,
                            name: "buf5".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 4,
                            name: "buf4".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                    ],
                    shader_type: ShaderType::Vertex,
                },
                ShaderFunction {
                    fn_name: "test_fragment".to_owned(),
                    binds: vec![
                        ShaderFunctionBind::Buffer {
                            index: 0,
                            name: "buf0".to_owned(),
                            data_type: "float".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 1,
                            name: "buf1".to_owned(),
                            data_type: "float2".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 2,
                            name: "buf2".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::Many,
                            immutable: false,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 3,
                            name: "buf3".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::One,
                            immutable: false,
                        },
                        ShaderFunctionBind::Texture {
                            index: 1,
                            name: "tex1".to_owned(),
                        },
                        ShaderFunctionBind::Buffer {
                            index: 5,
                            name: "buf5".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        ShaderFunctionBind::Buffer {
                            index: 4,
                            name: "buf4".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                    ],
                    shader_type: ShaderType::Fragment,
                },
            ];

            let shader_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("test_shader_src")
                .join("shader_fn")
                .canonicalize()
                .expect("Failed to canonicalize path to test_shader_src/deps directory");
            let shader_file = shader_dir.join("shaders.metal");
            let actual = generate_metal_ast(shader_file, |stdout| {
                parse_shader_functions_from_reader(stdout)
            });

            assert_eq!(actual, expected);
        }
    }
}
