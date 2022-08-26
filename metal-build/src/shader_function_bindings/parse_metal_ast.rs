use regex::{Captures, Regex};
use std::{
    collections::BTreeSet,
    fmt::Display,
    io::{BufRead, BufReader, Read},
    num::NonZeroU16,
};

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum BindType {
    One,
    Many,
}

// TODO: Could this just be From<BindType> for &str or something lighter than instead of Display.
impl Display for BindType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(
            match self {
                BindType::One => "Bind",
                BindType::Many => "BindMany",
            },
            f,
        )
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum Binds {
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
    AccelerationStructure {
        index: u8,
        name: String,
    },
}
impl Binds {
    pub const INVALID_INDEX: u8 = u8::MAX;
    fn with_new_index(self, index: u8) -> Self {
        use Binds::*;
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
            AccelerationStructure { name, .. } => AccelerationStructure { index, name },
        }
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum FunctionType {
    Vertex,
    Fragment,
    Compute,
}

impl FunctionType {
    pub const fn titlecase(&self) -> &'static str {
        match self {
            FunctionType::Vertex => "Vertex",
            FunctionType::Fragment => "Fragment",
            FunctionType::Compute => "Compute",
        }
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct FunctionConstant {
    pub name: String,
    pub data_type: String,
    pub index: u16,
}

impl FunctionConstant {
    pub const INVALID_INDEX: u16 = u16::MAX;
    pub fn new(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_owned(),
            data_type: data_type.to_owned(),
            index: Self::INVALID_INDEX,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct FunctionConstantAddress(u64);
impl FunctionConstantAddress {
    fn from_captures(c: &Captures<'_>) -> Self {
        Self(u64::from_str_radix(&c["address"], 16).expect("Failed to parse variable address"))
    }
}

struct FunctionConstants {
    function_constants: Vec<FunctionConstant>,
    function_constant_addresses: Vec<FunctionConstantAddress>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct FunctionConstantRef(NonZeroU16);

impl From<usize> for FunctionConstantRef {
    fn from(v: usize) -> Self {
        assert!(v < (u16::MAX as usize));
        Self(unsafe { NonZeroU16::new_unchecked((v + 1) as u16) })
    }
}
impl From<&FunctionConstantRef> for usize {
    fn from(v: &FunctionConstantRef) -> Self {
        (v.0.get() as usize) - 1
    }
}

impl FunctionConstants {
    fn new() -> Self {
        Self {
            function_constants: vec![],
            function_constant_addresses: vec![],
        }
    }
    fn push(&mut self, fn_const: FunctionConstant, fn_const_addr: FunctionConstantAddress) {
        self.function_constants.push(fn_const);
        self.function_constant_addresses.push(fn_const_addr);
    }
    fn get_ref(&mut self, addr: FunctionConstantAddress) -> Option<FunctionConstantRef> {
        for (i, &fn_const_addr) in self.function_constant_addresses.iter().enumerate() {
            if fn_const_addr == addr {
                return Some(FunctionConstantRef::from(i));
            }
        }
        None
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ParseFunction {
    pub fn_name: String,
    pub binds: Vec<Binds>,
    pub shader_type: Option<FunctionType>,
    pub referenced_function_constants: BTreeSet<FunctionConstantRef>,
}

impl ParseFunction {
    fn new(fn_name: &str) -> Self {
        Self {
            fn_name: fn_name.to_owned(),
            binds: vec![],
            shader_type: None,
            referenced_function_constants: BTreeSet::new(),
        }
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Function {
    pub fn_name: String,
    pub binds: Vec<Binds>,
    pub shader_type: FunctionType,
    pub referenced_function_constants: BTreeSet<FunctionConstantRef>,
}

impl From<ParseFunction> for Function {
    #[inline]
    fn from(
        ParseFunction {
            fn_name,
            binds,
            shader_type,
            referenced_function_constants,
        }: ParseFunction,
    ) -> Self {
        Self {
            fn_name,
            binds,
            shader_type: shader_type.expect("Failed to parse shader type"),
            referenced_function_constants,
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
pub fn parse_shader_functions_from_reader<R: Read>(
    shader_file_reader: R,
) -> (Vec<FunctionConstant>, Vec<Function>) {
    // FUNCTION REGULAR EXPRESSIONS
    // ----------------------------

    // TODO: Optimize Hex Address RegEx. Currently 0x\w+, could be 0x[0-9a-f]+

    // Example: |-TypedefDecl 0x13d841e60 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
    // Example: |-FunctionDecl 0x13d8ffd78 <line:26:14, col:79> col:21 test_vertex 'float4 (const constant int *)'
    let rx_any_top_level = Regex::new(r"^\|-[A-Z]").unwrap();

    // Example: |-FunctionDecl 0x14a1327a8 <line:9:1, line:11:15> line:9:8 main_vertex 'float4 (const constant packed_float4 *)'
    let rx_fn = Regex::new(
        r"^\|-FunctionDecl 0x[0-9a-f]+ <([^:]+)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+ (?P<fn_name>\w+) ",
    )
    .unwrap();

    // Example: | |-ParmVarDecl 0x14a132638 <line:10:5, col:29> col:29 yolo 'const constant packed_float4 *'
    // Example: | |-ParmVarDecl 0x116879d78 <line:7:5, col:21> col:21 tex0 'texture2d<half>':'metal::texture2d<half, metal::access::sample, void>'
    // Example: | |-ParmVarDecl 0x12614a0d0 <line:10:5, col:37> col:37 accelerationStructure 'metal::raytracing::instance_acceleration_structure':'metal::raytracing::_acceleration_structure<metal::raytracing::instancing>'
    let rx_fn_param = Regex::new(r"^\| (?P<last_child>[`|])-ParmVarDecl 0x[0-9a-f]+ <(line|col)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+( used)? (?P<name>\w+) '(?P<address_space>const constant |device |)(metal::)?(?P<data_type>\w[\w:<>, ]+)(?P<multiplicity> [*&]|)'").unwrap();

    // Example: | | `-MetalBufferIndexAttr 0x14a132698 <col:36, col:44>
    let rx_fn_param_metal_buffer_texture_index_attr = Regex::new(
        r"^\| \| (?P<last_child>[`|])-Metal(?P<buffer_or_texture>Buffer|Texture)IndexAttr ",
    )
    .unwrap();

    // Example: | |   `-IntegerLiteral 0x14a132570 <col:43> 'int' 0
    let rx_fn_param_metal_buffer_texture_index_attr_value = Regex::new(
        r"^\| \|   (?P<last_child>[`|])-IntegerLiteral 0x[0-9a-f]+ <(line|col)(:\d+)+> 'int' (?P<index>\d+)",
    )
    .unwrap();

    // Example: | `-MetalVertexAttr 0x14a132850 <line:8:3>
    // Example: | `-MetalFragmentAttr 0x14a132850 <line:8:3>
    let rx_fn_metal_shader_type_attr =
        Regex::new(r"^\| (?P<last_child>[`|])-Metal(?P<shader_type>Vertex|Fragment|Kernel)Attr ")
            .unwrap();

    // Example: | `-
    let rx_fn_last_child = Regex::new(r"^\| `-").unwrap();

    // Example: | | |         `-DeclRefExpr 0x13e12f4a0 <col:33> 'const constant bool' lvalue Var 0x13e0692d8 'HasDiffuse' 'const constant bool'
    // Example: | |         | | `-DeclRefExpr 0x12c932980 <col:12> 'const constant bool' lvalue Var 0x12c931ff0 'A_Bool' 'const constant bool'
    let rx_fn_constant_ref = Regex::new(r"^[\| ]+[`|]-DeclRefExpr 0x[0-9a-f]+ <(line|col)(:\d+)+> 'const constant .* Var 0x(?P<address>[0-9a-f]+) ").unwrap();

    // FUNCTION CONSTANT EXPRESSIONS
    // -----------------------------

    // Example: |-VarDecl 0x13a136af0 <line:7:1, col:26> col:26 HasAmbient 'const constant bool' constexpr
    let rx_var =
        Regex::new(r"^\|-VarDecl 0x(?P<address>[0-9a-f]+) <([^:]+)(:\d+)+, (line|col)(:\d+)+> (line|col)(:\d+)+ used (?P<name>\w+) 'const constant (metal::)?(?P<data_type>[\w:<>, ]+)'( |:'.* )constexpr$").unwrap();

    // Example: | `-MetalFunctionConstantAttr 0x13a136b50 <col:41, col:60>
    let rx_var_metal_func_const = Regex::new(r"^\| `-MetalFunctionConstantAttr ").unwrap();

    // Example: |   `-IntegerLiteral 0x13a136ac0 <col:59> 'int' 9
    let rx_var_metal_func_const_value =
        Regex::new(r"^\|   `-IntegerLiteral 0x[0-9a-f]+ <(line|col)(:\d+)+> 'int' (?P<index>\d+)")
            .unwrap();

    // COMMON REGULAR EXPRESSIONS
    // --------------------------

    // Example: | `-
    // Example: | | `-
    // Example: | | |`-
    let rx_last_child_of_any_level = Regex::new(r"^[\| ]+`-").unwrap();

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
        FindingRoot,
        Function(ParseFunction),
        FunctionParam(ParseFunction, ShaderFunctionParamInfo, FunctionChild),
        FunctionParamBufferOrTexture(ParseFunction, Binds, FunctionChild),
        Variable(FunctionConstant, FunctionConstantAddress),
        VariableValue(FunctionConstant, FunctionConstantAddress),
    }
    let mut fn_consts = FunctionConstants::new();
    let mut shader_fns: Vec<Function> = vec![];
    let mut parse_next_state = |state: State, l: String| {
        match state {
            State::FindingRoot => {
                if let Some(c) = rx_fn.captures(&l) {
                    return State::Function(ParseFunction::new(&c["fn_name"]));
                } else if let Some(c) = rx_var.captures(&l) {
                    return State::Variable(
                        FunctionConstant::new(&c["name"], &c["data_type"]),
                        FunctionConstantAddress::from_captures(&c),
                    );
                }
            }
            State::Function(mut fun) => {
                if let Some(c) = rx_fn_param.captures(&l) {
                    // TODO: Implement, this should determine immutability of buffers (render pipeline creation).
                    return State::FunctionParam(
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
                        "Vertex" => fun.shader_type = Some(FunctionType::Vertex),
                        "Fragment" => fun.shader_type = Some(FunctionType::Fragment),
                        "Kernel" => fun.shader_type = Some(FunctionType::Compute),
                        _ => panic!("Unexpected Metal function attribute ({shader_type})"),
                    }
                    // TODO: This may not be true for compute or object functions with parameterized attributes
                    // Example: [[object, max_total_threadgroups_per_mesh_grid(kMeshThreadgroups)]]
                    if FunctionChild::is_last_child(&c) {
                        shader_fns.push(fun.into());
                        return State::FindingRoot;
                    }
                } else if rx_fn_last_child.is_match(&l) {
                    if fun.shader_type.is_some() {
                        shader_fns.push(fun.into());
                    }
                    return State::FindingRoot;
                } else if let Some(c) = rx_fn_constant_ref.captures(&l) {
                    let addr = FunctionConstantAddress::from_captures(&c);
                    if let Some(index) = fn_consts.get_ref(addr) {
                        fun.referenced_function_constants.insert(index);
                    }
                } else if rx_any_top_level.is_match(&l) {
                    if let Some(c) = rx_fn.captures(&l) {
                        return State::Function(ParseFunction::new(&c["fn_name"]));
                    } else {
                        return State::FindingRoot;
                    }
                }
                return State::Function(fun);
            }
            State::FunctionParam(fun, info, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_texture_index_attr.captures(&l) {
                    debug_assert!(
                        FunctionChild::is_last_child(&c),
                        "Unsupported: Multiple function param attributes."
                    );
                    let buffer_or_texture = &c["buffer_or_texture"];
                    return State::FunctionParamBufferOrTexture(
                        fun,
                        if buffer_or_texture == "Buffer" {
                            let data_type = info.data_type.clone();
                            if data_type == "raytracing::instance_acceleration_structure"
                                || data_type == "raytracing::primitive_acceleration_structure"
                            {
                                Binds::AccelerationStructure {
                                    index: Binds::INVALID_INDEX,
                                    name: info.name,
                                }
                            } else {
                                Binds::Buffer {
                                    index: Binds::INVALID_INDEX,
                                    name: info.name,
                                    data_type: info.data_type,
                                    bind_type: if info.multiplicity == " *" {
                                        BindType::Many
                                    } else {
                                        debug_assert_eq!(
                                        info.multiplicity, " &",
                                        "Unexpected multiplicity, expected '&' or '*'. data_type: {data_type} Line: {l}"
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
                            }
                        } else {
                            debug_assert_eq!(buffer_or_texture, "Texture");
                            Binds::Texture {
                                index: Binds::INVALID_INDEX,
                                name: info.name,
                            }
                        },
                        fun_last_child,
                    );
                }
                if rx_last_child_of_any_level.is_match(&l) {
                    return match fun_last_child {
                        FunctionChild::Last => State::FindingRoot,
                        FunctionChild::NotLast => State::Function(fun),
                    };
                }
                return State::FunctionParam(fun, info, fun_last_child);
            }
            State::FunctionParamBufferOrTexture(mut fun, bind, fun_last_child) => {
                if let Some(c) = rx_fn_param_metal_buffer_texture_index_attr_value.captures(&l) {
                    debug_assert!(FunctionChild::is_last_child(&c), "Unexpected function param buffer attribute information to follow (why is this not the last child?)");
                    let index = c["index"].parse::<u8>().expect(&format!(
                        "Failed to parse function param buffer attribute index value ({l})"
                    ));
                    fun.binds.push(bind.with_new_index(index));
                    match fun_last_child {
                        FunctionChild::Last => {
                            shader_fns.push(fun.into());
                            return State::FindingRoot;
                        }
                        FunctionChild::NotLast => return State::Function(fun),
                    }
                }
                panic!("Unexpected function param buffer attribute information ({l})");
            }
            State::Variable(fn_const, fn_const_addr) => {
                if rx_var_metal_func_const.is_match(&l) {
                    return State::VariableValue(fn_const, fn_const_addr);
                }
                return State::FindingRoot;
            }
            State::VariableValue(mut fn_const, fn_const_addr) => {
                if let Some(c) = rx_var_metal_func_const_value.captures(&l) {
                    fn_const.index = c["index"].parse::<u16>().expect(&format!(
                        "Failed to parse function constant index value ({l})"
                    ));
                    fn_consts.push(fn_const, fn_const_addr);
                    return State::FindingRoot;
                }
                panic!("Unexpected function constant attribute information ({l})");
            }
        }
        state
    };

    let mut state = State::FindingRoot;
    let mut lines = BufReader::new(shader_file_reader).lines();
    while let Some(Ok(line)) = lines.next() {
        state = parse_next_state(state, line);
    }

    (fn_consts.function_constants, shader_fns)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    mod test_parse_shader_functions_from_reader {
        use super::*;

        fn test<const N_FN_CONSTS: usize, const N_FNS: usize>(
            input: &[u8],
            expected_fn_consts: [FunctionConstant; N_FN_CONSTS],
            expected_fns: [Function; N_FNS],
        ) {
            let (actual_fn_consts, actual_fns) = parse_shader_functions_from_reader(input);
            pretty_assertions::assert_eq!(&actual_fn_consts, &expected_fn_consts);
            pretty_assertions::assert_eq!(&actual_fns, &expected_fns);
        }

        #[test]
        fn test_fn_consts_simple() {
            /*
            constant constexpr bool   A_Bool   [[function_constant(9)]];
            constant constexpr float  A_Float  [[function_constant(2)]];
            constant constexpr float4 A_Float4 [[function_constant(4)]];
            constant constexpr uint   A_Uint   [[function_constant(1)]];
            */
            test(format!("\
TranslationUnitDecl 0x14c8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14c874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14c830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14c874928 <metal-build/test_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
|-VarDecl 0x1358cfdf0 <line:5:1, col:27> col:27 A_Bool 'const constant bool' constexpr
| `-MetalFunctionConstantAttr 0x1358cfe50 <col:40, col:59>
|   `-IntegerLiteral 0x1358cfda0 <col:58> 'int' 9
|-VarDecl 0x1358cff18 <line:6:1, col:27> col:27 used A_Float 'const constant float' constexpr
| `-MetalFunctionConstantAttr 0x1358cff78 <col:40, col:59>
|   `-IntegerLiteral 0x1358cfeb8 <col:58> 'int' 2
|-VarDecl 0x1358d0240 <line:7:1, col:27> col:27 A_Float4 'const constant float4':'float const constant __attribute__((ext_vector_type(4)))' constexpr
| `-MetalFunctionConstantAttr 0x1358d02a0 <col:40, col:59>
|   `-IntegerLiteral 0x1358d01d0 <col:58> 'int' 4
|-VarDecl 0x1358d0520 <line:8:1, col:27> col:27 used A_Uint 'const constant uint':'const constant unsigned int' constexpr
| `-MetalFunctionConstantAttr 0x1358d0580 <col:40, col:59>
|   `-IntegerLiteral 0x1358d04a8 <col:58> 'int' 1
`-<undeserialized declarations>
").as_bytes(),
                [
                    FunctionConstant {
                        name: "A_Float".to_owned(),
                        data_type: "float".to_owned(),
                        index: 2,
                    },
                    FunctionConstant {
                        name: "A_Uint".to_owned(),
                        data_type: "uint".to_owned(),
                        index: 1,
                    },
                ],
                []
            );
        }

        #[test]
        fn test_fn_consts() {
            /*
            constant constexpr bool   A_Bool   [[function_constant(9)]];
            constant constexpr float  A_Float  [[function_constant(2)]];
            constant constexpr float4 A_Float4 [[function_constant(4)]];
            constant constexpr uint   A_Uint   [[function_constant(1)]];

            [[vertex]]
            float4 test_vertex() {
                return A_Bool && A_Float4.x > 0 ? 1 : 0;
            }
            */
            test(format!("\
TranslationUnitDecl 0x1598302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x159874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x159830f20 '__metal_intersection_query_t'
|-ImportDecl 0x1598748f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x159931f50 <line:3:1, col:17> col:17 Namespace 0x1598749f0 'metal'
|-VarDecl 0x159931ff0 <line:5:1, col:27> col:27 used A_Bool 'const constant bool' constexpr
| `-MetalFunctionConstantAttr 0x159932050 <col:38, col:57>
|   `-IntegerLiteral 0x159931fa0 <col:56> 'int' 9
|-VarDecl 0x159932118 <line:6:1, col:27> col:27 used A_Float 'const constant float' constexpr
| `-MetalFunctionConstantAttr 0x159932178 <col:38, col:57>
|   `-IntegerLiteral 0x1599320b8 <col:56> 'int' 2
|-VarDecl 0x159932440 <line:7:1, col:27> col:27 used A_Float4 'const constant float4':'float const constant __attribute__((ext_vector_type(4)))' constexpr
| `-MetalFunctionConstantAttr 0x1599324a0 <col:38, col:57>
|   `-IntegerLiteral 0x1599323d0 <col:56> 'int' 4
|-VarDecl 0x159932720 <line:8:1, col:27> col:27 used A_Uint 'const constant uint':'const constant unsigned int' constexpr
| `-MetalFunctionConstantAttr 0x159932780 <col:38, col:57>
|   `-IntegerLiteral 0x1599326a8 <col:56> 'int' 1
|-VarDecl 0x159946e00 <line:9:1, col:27> col:27 A_Unused 'const constant ushort':'const constant unsigned short' constexpr
| `-MetalFunctionConstantAttr 0x159946e60 <col:38, col:57>
|   `-IntegerLiteral 0x159932990 <col:56> 'int' 3
|-FunctionDecl 0x159946f68 <line:12:1, line:14:1> line:12:8 test_vertex 'float4 ()'
| |-CompoundStmt 0x159947248 <col:22, line:14:1>
| | `-ReturnStmt 0x159947230 <line:13:5, col:43>
| |   `-ImplicitCastExpr 0x159947218 <col:12, col:43> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x159947200 <col:12, col:43> 'float' <IntegralToFloating>
| |       `-ConditionalOperator 0x1599471d0 <col:12, col:43> 'int'
| |         |-BinaryOperator 0x159947168 <col:12, col:35> 'bool' '&&'
| |         | |-ImplicitCastExpr 0x159947150 <col:12> 'bool' <LValueToRValue>
| |         | | `-DeclRefExpr 0x159947060 <col:12> 'const constant bool' lvalue Var 0x159931ff0 'A_Bool' 'const constant bool'
| |         | `-BinaryOperator 0x159947128 <col:22, col:35> 'bool' '>'
| |         |   |-ImplicitCastExpr 0x1599470f8 <col:22, col:31> 'float' <LValueToRValue>
| |         |   | `-ExtVectorElementExpr 0x1599470b0 <col:22, col:31> 'const constant float' lvalue vectorcomponent x
| |         |   |   `-DeclRefExpr 0x159947088 <col:22> 'const constant float4':'float const constant __attribute__((ext_vector_type(4)))' lvalue Var 0x159932440 'A_Float4' 'const constant float4':'float const constant __attribute__((ext_vector_type(4)))'
| |         |   `-ImplicitCastExpr 0x159947110 <col:35> 'float' <IntegralToFloating>
| |         |     `-IntegerLiteral 0x1599470d8 <col:35> 'int' 0
| |         |-IntegerLiteral 0x159947190 <col:39> 'int' 1
| |         `-IntegerLiteral 0x1599471b0 <col:43> 'int' 0
| `-MetalVertexAttr 0x159947008 <line:11:3>
|-FunctionDecl 0x159947280 <line:17:1, line:19:1> line:17:8 test_fragment 'float4 ()'
| |-CompoundStmt 0x159947598 <col:24, line:19:1>
| | `-ReturnStmt 0x159947580 <line:18:5, col:44>
| |   `-ImplicitCastExpr 0x159947568 <col:12, col:44> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x159947550 <col:12, col:44> 'float' <IntegralToFloating>
| |       `-ConditionalOperator 0x159947520 <col:12, col:44> 'int'
| |         |-BinaryOperator 0x1599474b8 <col:12, col:36> 'bool' '&&'
| |         | |-BinaryOperator 0x1599473f0 <col:12, col:22> 'bool' '<'
| |         | | |-ImplicitCastExpr 0x1599473c0 <col:12> 'float' <LValueToRValue>
| |         | | | `-DeclRefExpr 0x159947378 <col:12> 'const constant float' lvalue Var 0x159932118 'A_Float' 'const constant float'
| |         | | `-ImplicitCastExpr 0x1599473d8 <col:22> 'float' <IntegralToFloating>
| |         | |   `-IntegerLiteral 0x1599473a0 <col:22> 'int' 0
| |         | `-BinaryOperator 0x159947490 <col:27, col:36> 'bool' '>'
| |         |   |-ImplicitCastExpr 0x159947460 <col:27> 'uint':'unsigned int' <LValueToRValue>
| |         |   | `-DeclRefExpr 0x159947418 <col:27> 'const constant uint':'const constant unsigned int' lvalue Var 0x159932720 'A_Uint' 'const constant uint':'const constant unsigned int'
| |         |   `-ImplicitCastExpr 0x159947478 <col:36> 'unsigned int' <IntegralCast>
| |         |     `-IntegerLiteral 0x159947440 <col:36> 'int' 0
| |         |-IntegerLiteral 0x1599474e0 <col:40> 'int' 1
| |         `-IntegerLiteral 0x159947500 <col:44> 'int' 0
| `-MetalFragmentAttr 0x159947320 <line:16:3>
`-<undeserialized declarations>
").as_bytes(),
                [
                    FunctionConstant {
                        name: "A_Bool".to_owned(),
                        data_type: "bool".to_owned(),
                        index: 9,
                    },
                    FunctionConstant {
                        name: "A_Float".to_owned(),
                        data_type: "float".to_owned(),
                        index: 2,
                    },
                    FunctionConstant {
                        name: "A_Float4".to_owned(),
                        data_type: "float4".to_owned(),
                        index: 4,
                    },
                    FunctionConstant {
                        name: "A_Uint".to_owned(),
                        data_type: "uint".to_owned(),
                        index: 1,
                    },
                ],
                [
                    Function {
                        fn_name: "test_vertex".to_owned(),
                        binds: vec![],
                        shader_type: FunctionType::Vertex,
                        referenced_function_constants: BTreeSet::from([
                            FunctionConstantRef::from(0),
                            FunctionConstantRef::from(2),
                        ])
                    },
                    Function {
                        fn_name: "test_fragment".to_owned(),
                        binds: vec![],
                        shader_type: FunctionType::Fragment,
                        referenced_function_constants: BTreeSet::from([
                            FunctionConstantRef::from(1),
                            FunctionConstantRef::from(3),
                        ])
                    }
                ]
            );
        }

        #[test]
        fn test_non_binds() {
            /*
            [[vertex]]
            float4 test(uint vertex_id [[vertex_id]]) { return 0; }
            */
            test(format!("\
TranslationUnitDecl 0x14c8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14c874860 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x14c830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14c874928 <metal-build/test_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
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
[],
                [Function {
                    fn_name: "test".to_owned(),
                    binds: vec![],
                    shader_type: FunctionType::Vertex,
                    referenced_function_constants: BTreeSet::new()
                }]
            );
        }

        #[test]
        fn test_shader_types() {
            /*
            [[patch(quad, 4)]]
            [[vertex]]
            float4 test_vertex(){ return 0; }
            */
            test(b"\
TranslationUnitDecl 0x1570302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x157075460 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x157030f20 '__metal_intersection_query_t'
|-ImportDecl 0x1570754f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x157813150 <line:6:1, col:17> col:17 Namespace 0x157075628 'metal'
|-FunctionDecl 0x157813b08 <line:8:1, col:33> col:8 test 'float4 ()'
| |-CompoundStmt 0x157828080 <col:21, col:33>
| | `-ReturnStmt 0x157828068 <col:23, col:30>
| |   `-ImplicitCastExpr 0x157828050 <col:30> 'float4':'float __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x157828038 <col:30> 'float' <IntegralToFloating>
| |       `-IntegerLiteral 0x157828018 <col:30> 'int' 0
| |-MetalVertexAttr 0x157813ba8 <line:7:3>
| `-MetalPatchAttr 0x157813be8 <line:6:3, col:16> Quad
|   `-IntegerLiteral 0x157813a48 <col:15> 'int' 4
`-<undeserialized declarations>
",
                [],
                [Function {
                    fn_name: "test".to_owned(),
                    binds: vec![],
                    shader_type: FunctionType::Vertex,
                    referenced_function_constants: BTreeSet::new(),
                }]
            );

            /*
            [[kernel]]
            void test_vertex(){ }
            */
            test(b"\
TranslationUnitDecl 0x13c8302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x13c875460 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x13c830f20 '__metal_intersection_query_t'
|-ImportDecl 0x13c8754f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x13c933f50 <line:6:1, col:17> col:17 Namespace 0x13c875628 'metal'
|-FunctionDecl 0x13c934898 <line:7:1, col:21> col:6 test 'void ()'
| |-CompoundStmt 0x13c934990 <col:19, col:21>
| `-MetalKernelAttr 0x13c934938 <line:6:3>
`-<undeserialized declarations>
",
                [],
                [Function {
                    fn_name: "test".to_owned(),
                    binds: vec![],
                    shader_type: FunctionType::Compute,
                    referenced_function_constants: BTreeSet::new(),
                }]
            );
        }

        #[test]
        fn test_shader_with_raytracing() {
            /*
            #include <metal_stdlib>
            using namespace metal;

            using raytracing::instance_acceleration_structure;

            [[fragment]]
            half4 test(
                instance_acceleration_structure accelerationStructure [[buffer(0)]]
            ) {
                return 0;
            }
            */
            test(b"\
TranslationUnitDecl 0x1260302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x126075660 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x126030f20 '__metal_intersection_query_t'
|-ImportDecl 0x1260756f0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x126134150 <line:6:1, col:17> col:17 Namespace 0x126075828 'metal'
|-UsingDirectiveDecl 0x1261349e8 <metal-build/test_src/shader_fn/shaders.metal:4:1, col:17> col:17 Namespace 0x126075828 'metal'
|-UsingDecl 0x126149d48 <line:5:1, col:19> col:19 raytracing::instance_acceleration_structure
|-UsingShadowDecl 0x126149d98 <col:19> col:19 implicit TypeAlias 0x126134ae8 'instance_acceleration_structure'
| `-TypedefType 0x126149c10 'metal::raytracing::instance_acceleration_structure' sugar imported
|   |-TypeAlias 0x126134ae8 'instance_acceleration_structure'
|   `-TemplateSpecializationType 0x126149b80 'acceleration_structure<metal::raytracing::instancing>' sugar imported alias acceleration_structure
|     |-TemplateArgument type 'metal::raytracing::instancing'
|     |-TemplateSpecializationType 0x126149b30 '_acceleration_structure<metal::raytracing::instancing>' sugar imported _acceleration_structure
|     | |-TemplateArgument type 'metal::raytracing::instancing':'metal::raytracing::instancing'
|     | `-RecordType 0x126149b10 'metal::raytracing::_acceleration_structure<metal::raytracing::instancing>' imported
|     |   `-ClassTemplateSpecialization 0x126149970 '_acceleration_structure'
|     `-RecordType 0x126149b10 'metal::raytracing::_acceleration_structure<metal::raytracing::instancing>' imported
|       `-ClassTemplateSpecialization 0x126149970 '_acceleration_structure'
|-FunctionDecl 0x12614a4b8 <line:8:1, line:13:1> line:8:7 test 'half4 (float4, metal::raytracing::instance_acceleration_structure)'
| |-ParmVarDecl 0x12614a0d0 <line:10:5, col:37> col:37 accelerationStructure 'metal::raytracing::instance_acceleration_structure':'metal::raytracing::_acceleration_structure<metal::raytracing::instancing>'
| | `-MetalBufferIndexAttr 0x12614a378 <col:61, col:69>
| |   `-IntegerLiteral 0x12614a0a0 <col:68> 'int' 0
| |-CompoundStmt 0x12614a628 <line:11:3, line:13:1>
| | `-ReturnStmt 0x12614a610 <line:12:5, col:12>
| |   `-ImplicitCastExpr 0x12614a5f8 <col:12> 'half4':'half __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x12614a5e0 <col:12> 'half' <IntegralToFloating>
| |       `-IntegerLiteral 0x12614a5c0 <col:12> 'int' 0
| `-MetalFragmentAttr 0x12614a568 <line:7:3>
`-<undeserialized declarations>
",
                [],
                [Function {
                    fn_name: "test".to_owned(),
                    binds: vec![
                        Binds::AccelerationStructure {
                            index: 0,
                            name: "accelerationStructure".to_owned(),
                        },
                    ],
                    shader_type: FunctionType::Fragment,
                    referenced_function_constants: BTreeSet::new(),
                }],
            );

            /*
            #include <metal_stdlib>
            using namespace metal;

            using raytracing::primitive_acceleration_structure;

            [[fragment]]
            half4 test(
                primitive_acceleration_structure accelerationStructure [[buffer(1)]]
            ) {
                return 0;
            }
            */
            test(b"\
TranslationUnitDecl 0x1428302e8 <<invalid sloc>> <invalid sloc>
|-TypedefDecl 0x14302fc60 <<invalid sloc>> <invalid sloc> implicit __metal_intersection_query_t '__metal_intersection_query_t'
| `-BuiltinType 0x142830f20 '__metal_intersection_query_t'
|-ImportDecl 0x14302fcf0 <<built-in>:1:1> col:1 implicit metal_types
|-UsingDirectiveDecl 0x1430ecd50 <line:2:1, col:17> col:17 Namespace 0x14302fdf0 'metal'
|-UsingDecl 0x1430f5e68 <line:4:1, col:19> col:19 raytracing::primitive_acceleration_structure
|-UsingShadowDecl 0x1430f5eb8 <col:19> col:19 implicit TypeAlias 0x1430ece50 'primitive_acceleration_structure'
| `-TypedefType 0x1430f5d30 'metal::raytracing::primitive_acceleration_structure' sugar imported
|   |-TypeAlias 0x1430ece50 'primitive_acceleration_structure'
|   `-TemplateSpecializationType 0x1430f5ce0 'acceleration_structure<>' sugar imported alias acceleration_structure
|     |-TemplateSpecializationType 0x1430f5cb0 '_acceleration_structure<>' sugar imported _acceleration_structure
|     | `-RecordType 0x1430f5c90 'metal::raytracing::_acceleration_structure<>' imported
|     |   `-ClassTemplateSpecialization 0x1430ed718 '_acceleration_structure'
|     `-RecordType 0x1430f5c90 'metal::raytracing::_acceleration_structure<>' imported
|       `-ClassTemplateSpecialization 0x1430ed718 '_acceleration_structure'
|-FunctionDecl 0x1430f6658 <line:7:1, line:11:1> line:7:7 test 'half4 (metal::raytracing::primitive_acceleration_structure)'
| |-ParmVarDecl 0x1430f6130 <line:8:5, col:38> col:38 accelerationStructure 'metal::raytracing::primitive_acceleration_structure':'metal::raytracing::_acceleration_structure<>'
| | `-MetalBufferIndexAttr 0x1430f6548 <col:62, col:70>
| |   `-IntegerLiteral 0x1430f6100 <col:69> 'int' 1
| |-CompoundStmt 0x1430f67c0 <line:9:3, line:11:1>
| | `-ReturnStmt 0x1430f67a8 <line:10:5, col:12>
| |   `-ImplicitCastExpr 0x1430f6790 <col:12> 'half4':'half __attribute__((ext_vector_type(4)))' <VectorSplat>
| |     `-ImplicitCastExpr 0x1430f6778 <col:12> 'half' <IntegralToFloating>
| |       `-IntegerLiteral 0x1430f6758 <col:12> 'int' 0
| `-MetalFragmentAttr 0x1430f6700 <line:6:3>
`-<undeserialized declarations>
            ",
                [],
                [Function {
                    fn_name: "test".to_owned(),
                    binds: vec![
                        Binds::AccelerationStructure {
                            index: 1,
                            name: "accelerationStructure".to_owned(),
                        },
                    ],
                    shader_type: FunctionType::Fragment,
                    referenced_function_constants: BTreeSet::new(),
                }],
            );
        }

        #[test]
        fn test_no_binds() {
            for path in ["line", "proj-2-transformations/src/shaders.metal"] {
                for (metal_attr, expected_shader_type) in [
                    ("MetalVertexAttr", FunctionType::Vertex),
                    ("MetalFragmentAttr", FunctionType::Fragment),
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
[],
                    [Function {
                        fn_name: "test".to_owned(),
                        binds: vec![],
                        shader_type: expected_shader_type,
                        referenced_function_constants: BTreeSet::new()
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
|-ImportDecl 0x14d874928 <metal-build/test_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
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
                    [],
                    [Function {
                        fn_name: "test".to_owned(),
                        binds: vec![
                            Binds::Buffer { index: 0, name: "buf0".to_owned(), data_type: "float4x4".to_owned(), bind_type: expected_bind_type, immutable: expected_immutable }
                        ],
                        shader_type: FunctionType::Vertex,
                        referenced_function_constants: BTreeSet::new()
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
                [],
                [
                    Function {
                        fn_name: "test".to_owned(),
                        binds: vec![
                            Binds::Buffer { index: 0, name: "buf0".to_owned(), data_type: "float".to_owned(), bind_type: BindType::Many, immutable: true },
                            Binds::Buffer { index: 1, name: "buf1".to_owned(), data_type: "float2".to_owned(), bind_type: BindType::One, immutable: true },
                            Binds::Buffer { index: 2, name: "buf2".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::Many, immutable: false },
                            Binds::Buffer { index: 3, name: "buf3".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::One, immutable: false },
                            Binds::Texture { index: 1, name: "tex1".to_owned() },
                            Binds::Buffer { index: 5, name: "buf5".to_owned(), data_type: "TestStruct".to_owned(), bind_type: BindType::One, immutable: true },
                            Binds::Buffer { index: 4, name: "buf4".to_owned(), data_type: "TestStruct".to_owned(), bind_type: BindType::Many, immutable: true },
                        ],
                        shader_type: FunctionType::Vertex,
                        referenced_function_constants: BTreeSet::new()
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
                    [],
                    [
                        Function {
                            shader_type: FunctionType::Fragment,
                            fn_name: "test".to_owned(),
                            binds: vec![
                                Binds::Texture { index: 0, name: "tex0".to_owned() },
                            ],
                            referenced_function_constants: BTreeSet::new()
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
                [],
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
|-ImportDecl 0x13d841f28 <metal-build/test_src/shader_fn/shaders.metal:1:1> col:1 implicit metal_stdlib
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
                [],
                [
                    Function {
                        fn_name: "test_vertex".to_owned(),
                        binds: vec![
                            Binds::Buffer { index: 0, name: "buf0".to_owned(), data_type: "int".to_owned(), bind_type: BindType::Many, immutable: true },
                        ],
                        shader_type: FunctionType::Vertex,
                        referenced_function_constants: BTreeSet::new()
                    },
                    Function {
                        fn_name: "test_fragment".to_owned(),
                        binds: vec![
                            Binds::Buffer { index: 1, name: "buf1".to_owned(), data_type: "float3".to_owned(), bind_type: BindType::Many, immutable: false },
                        ],
                        shader_type: FunctionType::Fragment,
                        referenced_function_constants: BTreeSet::new()
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
            let expected_fns = vec![
                Function {
                    fn_name: "test_vertex".to_owned(),
                    binds: vec![
                        Binds::Buffer {
                            index: 0,
                            name: "buf0".to_owned(),
                            data_type: "float".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                        Binds::Buffer {
                            index: 1,
                            name: "buf1".to_owned(),
                            data_type: "float2".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        Binds::Buffer {
                            index: 2,
                            name: "buf2".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::Many,
                            immutable: false,
                        },
                        Binds::AccelerationStructure {
                            index: 6,
                            name: "accelerationStructure".to_owned(),
                        },
                        Binds::Buffer {
                            index: 3,
                            name: "buf3".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::One,
                            immutable: false,
                        },
                        Binds::Texture {
                            index: 1,
                            name: "tex1".to_owned(),
                        },
                        Binds::Buffer {
                            index: 5,
                            name: "buf5".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        Binds::Buffer {
                            index: 4,
                            name: "buf4".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                    ],
                    shader_type: FunctionType::Vertex,
                    referenced_function_constants: BTreeSet::from([FunctionConstantRef::from(0)]),
                },
                Function {
                    fn_name: "test_fragment".to_owned(),
                    binds: vec![
                        Binds::Buffer {
                            index: 0,
                            name: "buf0".to_owned(),
                            data_type: "float".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                        Binds::Buffer {
                            index: 1,
                            name: "buf1".to_owned(),
                            data_type: "float2".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        Binds::AccelerationStructure {
                            index: 6,
                            name: "accelerationStructure".to_owned(),
                        },
                        Binds::Buffer {
                            index: 2,
                            name: "buf2".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::Many,
                            immutable: false,
                        },
                        Binds::Buffer {
                            index: 3,
                            name: "buf3".to_owned(),
                            data_type: "float3".to_owned(),
                            bind_type: BindType::One,
                            immutable: false,
                        },
                        Binds::Texture {
                            index: 1,
                            name: "tex1".to_owned(),
                        },
                        Binds::Buffer {
                            index: 5,
                            name: "buf5".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::One,
                            immutable: true,
                        },
                        Binds::Buffer {
                            index: 4,
                            name: "buf4".to_owned(),
                            data_type: "TestStruct".to_owned(),
                            bind_type: BindType::Many,
                            immutable: true,
                        },
                    ],
                    shader_type: FunctionType::Fragment,
                    referenced_function_constants: BTreeSet::from([
                        FunctionConstantRef::from(1),
                        FunctionConstantRef::from(2),
                    ]),
                },
            ];

            let shader_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("test_src")
                .join("shader_fn")
                .canonicalize()
                .expect("Failed to canonicalize path to test_src/deps directory");
            let shader_file = shader_dir.join("shaders.metal");
            let (actual_fn_consts, actual_fns) = generate_metal_ast(shader_file, |stdout| {
                parse_shader_functions_from_reader(stdout)
            });

            pretty_assertions::assert_eq!(
                actual_fn_consts,
                &[
                    FunctionConstant {
                        name: "A_Bool".to_owned(),
                        data_type: "bool".to_owned(),
                        index: 0,
                    },
                    FunctionConstant {
                        name: "A_Float".to_owned(),
                        data_type: "float".to_owned(),
                        index: 1,
                    },
                    FunctionConstant {
                        name: "A_Uint".to_owned(),
                        data_type: "uint".to_owned(),
                        index: 3,
                    },
                ]
            );
            pretty_assertions::assert_eq!(actual_fns, expected_fns);
        }
    }
}
