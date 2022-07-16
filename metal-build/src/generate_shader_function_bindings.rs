use super::parse_shader_function::{parse_shader_functions, ShaderFunction, ShaderFunctionBind};
use std::{io::Write, path::Path};

pub fn generate_shader_function_bindings<P: AsRef<Path>, W: Write>(shader_file: P, writer: &mut W) {
    let mut w = |s: &str| {
        writer
            .write_all(s.as_bytes())
            .expect("Unable to write shader_bindings.rs file (shader function bindings)");
    };
    w(r#"
/**************************************************************************************************
 Shader function generations
***************************************************************************************************/
"#);
    for ShaderFunction {
        fn_name,
        binds,
        shader_type,
    } in parse_shader_functions(shader_file)
    {
        w(&format!(
            r#"
pub struct {fn_name}_binds<'a> {{"#
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
                ShaderFunctionBind::Buffer {
                    index,
                    name,
                    data_type,
                    bind_type,
                    // TODO: Implement marking buffers as immutible
                    immutable: _,
                } => {
                    w(&format!(
                        r#"
    {name}: Bind{bind_type}<'a, {index}, {data_type}>,"#
                    ));
                }
                ShaderFunctionBind::Texture { .. } => todo!(),
            }
        }
        let shader_type_lowercase = shader_type.lowercase();
        let shader_type_titlecase = shader_type.titlecase();
        w(&format!(
            r#"
}}

pub impl<'c> {shader_type_titlecase}ShaderBinds for {fn_name}_binds<'c> {{
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
                ShaderFunctionBind::Buffer { name, .. }
                | ShaderFunctionBind::Texture { name, .. } => w(&format!(
                    r#"
        self.{name}.encode_for_{shader_type_lowercase}(encoder);"#
                )),
            }
        }
        w(r#"
    }
}
"#);
    }
}
