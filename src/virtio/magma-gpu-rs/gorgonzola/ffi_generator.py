# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import typing as T

from gorgonzola.code_writer import CodeWriter
from gorgonzola.common import (
    SINGLE_INDENT,
    MAX_LINE_LENGTH,
    CommonGenerator,
    ApiAspectMask,
    Direction,
    CommandKind,
    GorgonzolaApiObject,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    GorgonzolaConstant,
    GorgonzolaConstantArray,
    GorgonzolaEnum,
    GorgonzolaFlag,
    GorgonzolaBaseEnum,
    GorgonzolaCommand,
    GorgonzolaStructMember,
    GorgonzolaTypeInstance,
    BaseRustTypeInfo,
    to_rust_protocol_type,
    ApiData,
)
from gorgonzola.post_process import PostProcessedApiData, struct_has_api_objects
from gorgonzola.utils import to_prefixed_snake_case, to_snake_case

COMMON_RUST_IMPORTS = """\
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::boxed::Box;
use std::convert::TryInto;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::null_mut;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::{Arc, Mutex};

use libc::{EINVAL, ESRCH};
use log::{debug, error};
"""

RETURN_RESULT_FN = """\
fn return_result(result: Result<()>) -> i32 {{
    match result {{
        Ok(()) => NO_ERROR,
        Err(e) => {{
            debug!("error: {{:?}}", e);
            e.into()
        }}
    }}
}}
"""

RETURN_ON_ERROR_MACRO = """\
macro_rules! return_on_error {
    ($result:expr) => {
        match $result {
            Ok(t) => t,
            Err(e) => {
                debug!("error: {:?}", e);
                return e.into();
            }
        }
    };
}
"""

ASSIGN_FFI_TYPE = """\
#[allow(non_camel_case_types)]
pub type {ffi_name} = {rust_name};
"""

FFI_FUNCTION_SINGLE = """\
#[no_mangle]
pub unsafe extern "C" fn {name}({args}) -> {ret} {{
    catch_unwind(AssertUnwindSafe(|| {{
        debug!("call: {name}");
{body}    }}))
    .{unwrap_expr}
}}
"""

FFI_FUNCTION_MULTI = """\
#[no_mangle]
pub unsafe extern "C" fn {name}(
    {args}
) -> {ret} {{
    catch_unwind(AssertUnwindSafe(|| {{
        debug!("call: {name}");
{body}    }}))
    .{unwrap_expr}
}}
"""

UNWRAP_OR_ESRCH = 'unwrap_or(-ESRCH)'

DESTROY_MUT_OBJECT = """\
        if !(*{param_var}).is_null() {{
            let _ = unsafe {{ Box::from_raw(*{param_var}) }};
            *{param_var} = null_mut();
        }}
        NO_ERROR
"""

DESTROY_CONST_OBJECT = """\
        let _ = unsafe {{ Box::from_raw({param_var} as *mut _) }};
        NO_ERROR
"""

INVOKE_STATIC_FUNCTION = """\
        let result = {call_target};
        let items = return_on_error!(result);
        *{count_var} = items.len().try_into().unwrap();
        if *{count_var} > {max_len} {{
            return -EINVAL;
        }}

        let out_slice = from_raw_parts_mut({arr_var}, {max_len} as usize);
        for (i, item) in items.into_iter().enumerate() {{
            out_slice[i] = Box::into_raw(Box::new(item)) as _;
        }}

        NO_ERROR
"""

INVOKE_DISPATCHABLE = """\
{conversions}\
        let result = {dispatchable}.{method_name}({args});
        let res = return_on_error!(result);
        *{out_var} = Box::into_raw(Box::new(res)) as _;
        NO_ERROR
"""

INVOKE_METHOD_WITH_OUT_VAR = """\
{conversions}\
        let result = {recv_var}.{method_name}({args});
        *{out_var} = return_on_error!(result);
        NO_ERROR
"""

INVOKE_METHOD_COPY_OUT_VAR = """\
{conversions}\
        let result = {recv_var}.{method_name}({args});
        *{out_var} = *result;
"""


INVOKE_METHOD = """\
{conversions}\
        let result = {recv_var}.{method_name}({args});
        return_result(result)
"""



def _to_ffi_type_str(type_inst: GorgonzolaTypeInstance, api_data: ApiData) -> str:
    match type_inst:
        case GorgonzolaApiObject(name=name):
            return f'*mut {to_prefixed_snake_case(name, api_data.api_name)}'
        case GorgonzolaStruct(name=name, has_api_object=True):
            return to_snake_case(name)
        case GorgonzolaStruct(name=name):
            return name
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            return f'[{_to_ffi_type_str(elem, api_data)}; {size}]'
        case _:
            return to_rust_protocol_type(type_inst)


def to_ffi_struct_field(member: GorgonzolaStructMember, api_data: ApiData) -> str:
    if member.is_pointer:
        return f'*mut {_to_ffi_type_str(member.type, api_data)}'
    return _to_ffi_type_str(member.type, api_data)




def to_rust_conversion(struct_obj: GorgonzolaStruct, api_data: ApiData, w: CodeWriter) -> None:
    rust_name = struct_obj.name
    ffi_name = to_snake_case(rust_name)

    w.set_indent_multiple(0)
    w.write_line(f'impl {ffi_name} {{')
    w.set_indent_multiple(1)
    w.write_line(f'pub fn to_rust(&self) -> {rust_name} {{')

    handle_conversions = api_data.ffi_config.get('handle_conversions', {})
    for m in struct_obj.members:
        match m.type:
            case GorgonzolaApiObject(name=obj_name) if obj_name in handle_conversions:
                conv_cfg = handle_conversions[obj_name]
                match conv_cfg:
                    case str(conv_str):
                        conv_expr = f'unsafe {{ (*self.{m.name}).{conv_str} }}'
                        default_val = '0'
                    case dict():
                        conv_expr = conv_cfg.get('expr', f'unsafe {{ (*self.{m.name}).clone() }}')
                        default_val = conv_cfg.get('default', '0')
                w.set_indent_multiple(2)
                w.write_line(f'let {m.name} = if !self.{m.name}.is_null() {{')
                w.set_indent_multiple(3)
                w.write_line(f'{conv_expr}')
                w.set_indent_multiple(2)
                w.write_line('} else {')
                w.set_indent_multiple(3)
                w.write_line(f'{default_val}')
                w.set_indent_multiple(2)
                w.write_line('};')
            case GorgonzolaApiObject():
                w.set_indent_multiple(2)
                w.write_line(f'let {m.name} = 0;')
            case GorgonzolaConstantArray(
                element_type=GorgonzolaApiObject(name=elem_name), array_size_name=size_name
            ) if elem_name in handle_conversions:
                conv_cfg = handle_conversions[elem_name]
                match conv_cfg:
                    case str(conv_str):
                        rust_elem_type = 'u32'
                        conv_expr = f'unsafe {{ (*ptr).{conv_str} }}'
                        default_val = '0'
                    case dict():
                        rust_elem_type = conv_cfg.get('rust_type', 'u32')
                        conv_expr = conv_cfg.get('expr', 'unsafe { (*ptr).clone() }')
                        default_val = conv_cfg.get('default', '0')
                count_field = f'num_{m.name}'
                has_count = any(other.name == count_field for other in struct_obj.members)
                limit_expr = (
                    f'(self.{count_field} as usize).min({size_name})'
                    if has_count
                    else f'{size_name} as usize'
                )
                w.set_indent_multiple(2)
                if default_val == '0':
                    w.write_line(f'let mut {m.name} = [0u32; {size_name}];')
                else:
                    w.write_line(f'let mut {m.name}: [{rust_elem_type}; {size_name}] = Default::default();')
                w.write_line(f'let limit_{m.name} = {limit_expr};')
                w.write_line(f'for i in 0..limit_{m.name} {{')
                w.set_indent_multiple(3)
                w.write_line(f'let ptr = self.{m.name}[i];')
                w.write_line('if !ptr.is_null() {')
                w.set_indent_multiple(4)
                w.write_line(f'{m.name}[i] = {conv_expr};')
                w.set_indent_multiple(3)
                w.write_line('}')
                w.set_indent_multiple(2)
                w.write_line('}')
            case GorgonzolaConstantArray(
                element_type=GorgonzolaApiObject(), array_size_name=size_name
            ):
                w.set_indent_multiple(2)
                w.write_line(f'let {m.name} = [0u32; {size_name}];')
            case _:
                pass

    w.set_indent_multiple(2)
    w.write_line(f'{rust_name} {{')

    stype_header = api_data.ffi_config.get('structure_type_header', '')
    if isinstance(struct_obj, GorgonzolaExtensibleStruct):
        w.set_indent_multiple(3)
        w.write_line(f'header: {stype_header} {{')
        w.set_indent_multiple(4)
        w.write_line('stype: self.header.stype,')
        w.write_line(f'size: std::mem::size_of::<{rust_name}>() as u32,')
        w.write_line('p_next: self.header.p_next,')
        w.set_indent_multiple(3)
        w.write_line('},')

    for m in struct_obj.members:
        match m.type:
            case GorgonzolaApiObject():
                w.set_indent_multiple(3)
                w.write_line(f'{m.name},')
            case GorgonzolaConstantArray(element_type=GorgonzolaApiObject()):
                w.set_indent_multiple(3)
                w.write_line(f'{m.name},')
            case GorgonzolaStruct(has_api_object=True):
                w.set_indent_multiple(3)
                w.write_line(f'{m.name}: self.{m.name}.to_rust(),')
            case _:
                w.set_indent_multiple(3)
                w.write_line(f'{m.name}: self.{m.name},')

    w.set_indent_multiple(3)
    w.write_line('..Default::default()')
    w.set_indent_multiple(2)
    w.write_line('}')
    w.set_indent_multiple(1)
    w.write_line('}')
    w.set_indent_multiple(0)
    w.write_line('}')


def to_rust_ffi_type(type_inst: GorgonzolaTypeInstance, api_name: str) -> str:
    match type_inst:
        case BaseRustTypeInfo(name='void'):
            return '()'
        case BaseRustTypeInfo(name=name):
            return name
        case GorgonzolaApiObject(name=name):
            return to_prefixed_snake_case(name, api_name)
        case GorgonzolaBaseEnum(name=name) | GorgonzolaStruct(name=name):
            return to_snake_case(name)
        case GorgonzolaConstantArray(element_type=elem):
            return to_rust_ffi_type(elem, api_name)
        case _:
            return '()'


def to_rust_ffi_arg(
    type_inst: GorgonzolaTypeInstance, name: str, is_const: bool, api_name: str
) -> str:
    match type_inst:
        case BaseRustTypeInfo(name=ty):
            return f'{name}: {ty}' if is_const else f'{name}: &mut {ty}'

        case GorgonzolaBaseEnum(name=n):
            ty = to_snake_case(n)
            return f'{name}: {ty}' if is_const else f'{name}: *mut {ty}'

        case GorgonzolaApiObject(name=n):
            ty = to_prefixed_snake_case(n, api_name)
            return f'{name}: &mut {ty}' if is_const else f'{name}: &mut *mut {ty}'

        case GorgonzolaConstantArray(element_type=GorgonzolaApiObject(name=n)):
            ty = to_prefixed_snake_case(n, api_name)
            return f'{name}: &mut *const {ty}' if is_const else f'{name}: &mut *mut {ty}'

        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            elem_ty = to_rust_ffi_type(elem, api_name)
            return (
                f'{name}: &[{elem_ty}; {size}]' if is_const else f'{name}: &mut [{elem_ty}; {size}]'
            )

        case GorgonzolaStruct(name=n):
            ty = to_snake_case(n)
            return f'{name}: &{ty}' if is_const else f'{name}: &mut {ty}'

        case _:
            ty = to_rust_ffi_type(type_inst, api_name)
            return f'{name}: &{ty}' if is_const else f'{name}: &mut {ty}'


def to_rust_ffi_member_arg(
    member: GorgonzolaStructMember, api_name: str, is_const: T.Optional[bool] = None
) -> str:
    if is_const is None:
        is_const = member.direction == Direction.IN
    return to_rust_ffi_arg(member.type, member.name, is_const, api_name)


class FFIGenerator(CommonGenerator):
    ASPECT_MASK = ApiAspectMask.FFI

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        rust_type = to_rust_protocol_type(self.api_data.resolve_type_name(obj.type_name))
        self.code_writer.write_line(f'const {obj.name}: {rust_type} = {obj.value};')

    def emit_api_object(self, obj: GorgonzolaApiObject) -> None:
        self._emit_type(obj)

    def emit_enum(self, obj: GorgonzolaEnum) -> None:
        self._emit_type(obj)

    def emit_flag(self, obj: GorgonzolaFlag) -> None:
        self._emit_type(obj)

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        self._emit_type(obj)

    def _emit_type(self, obj: GorgonzolaTypeInstance) -> None:
        ffi_config = self.api_data.ffi_config
        exclude_imports = set(ffi_config.get('exclude_imports', []))
        if obj.name in exclude_imports:
            return

        if isinstance(obj, GorgonzolaStruct) and struct_has_api_objects(obj, self.api_data):
            self._emit_struct_with_api_objects(obj)
            return

        if isinstance(obj, GorgonzolaApiObject):
            ffi_name = to_prefixed_snake_case(obj.name, self.api_name.snake)
        else:
            ffi_name = to_snake_case(obj.name)
        if ffi_name != obj.name:
            self.code_writer.write_line('#[allow(non_camel_case_types)]')
            self.code_writer.write_line(f'pub type {ffi_name} = {obj.name};')
            self.code_writer.write_line()

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        if obj.manual:
            return
        ffi_config = self.api_data.ffi_config
        status_types = set(ffi_config.get('status_types', []))

        api_name = self.api_name.snake
        args = [to_rust_ffi_member_arg(p, api_name) for p in obj.inputs]
        ret_type = to_rust_ffi_type(obj.ret, api_name)
        if (
            isinstance(obj.ret, GorgonzolaEnum)
            or obj.ret.name in status_types
            or ret_type in status_types
        ):
            ret_type = 'i32'

        body = self._generate_ffi_body(obj.name, obj, ret_type)
        func_name = to_snake_case(obj.name)
        unwrap_expr = 'unwrap_or(())' if ret_type == '()' else UNWRAP_OR_ESRCH

        single_line_fn = f'pub unsafe extern "C" fn {func_name}({", ".join(args)}) -> {ret_type} {{'
        if len(single_line_fn) <= MAX_LINE_LENGTH:
            fn_template = FFI_FUNCTION_SINGLE
            args_str = ", ".join(args)
        else:
            fn_template = FFI_FUNCTION_MULTI
            args_str = f',\n{SINGLE_INDENT}'.join(args) + (',' if args else '')

        self.code_writer.write_raw(
            fn_template.format(
                name=func_name,
                args=args_str,
                ret=ret_type,
                body=body,
                unwrap_expr=unwrap_expr,
            ).strip()
        )
        self.code_writer.write_line()

    def _generate_ffi_body(self, cmd_name: str, cmd: GorgonzolaCommand, ret_type: str) -> str:
        body = self._generate_ffi_body_impl(cmd_name, cmd)
        if ret_type == '()' and body.rstrip().endswith('NO_ERROR'):
            lines = body.rstrip().split('\n')
            if lines[-1].strip() == 'NO_ERROR':
                body = '\n'.join(lines[:-1]) + '\n' if len(lines) > 1 else ''
        return body

    def _generate_ffi_body_impl(self, cmd_name: str, cmd: GorgonzolaCommand) -> str:
        match cmd.kind:
            case CommandKind.DESTRUCTOR:
                return self._generate_destructor_body(cmd)
            case CommandKind.STATIC_FUNCTION:
                return self._generate_static_function_body(cmd_name, cmd)
            case CommandKind.OBJECT_CREATION:
                return self._generate_object_creation_body(cmd_name, cmd)
            case CommandKind.METHOD_CALL:
                return self._generate_method_call_body(cmd_name, cmd)

    def _generate_destructor_body(self, cmd: GorgonzolaCommand) -> str:
        if cmd.inputs_mut:
            return DESTROY_MUT_OBJECT.format(param_var=cmd.inputs_mut[0].name)
        if cmd.inputs_const:
            return DESTROY_CONST_OBJECT.format(param_var=cmd.inputs_const[0].name)
        return '        NO_ERROR\n'

    def _generate_static_function_body(self, cmd_name: str, cmd: GorgonzolaCommand) -> str:
        if len(cmd.inputs_mut) >= 2:
            arr_param = cmd.inputs_mut[0]
            count_param = cmd.inputs_mut[1]
            api_prefix = f'{self.api_name.snake.lower()}_'
            call_target = f'{cmd.target or to_snake_case(cmd_name).removeprefix(api_prefix)}()'
            max_len = (
                arr_param.type.array_size_name
                if isinstance(arr_param.type, GorgonzolaConstantArray)
                else '8'
            )
            return INVOKE_STATIC_FUNCTION.format(
                call_target=call_target,
                count_var=count_param.name,
                max_len=max_len,
                arr_var=arr_param.name,
            )
        return '        NO_ERROR\n'

    def _resolve_method_invocation(
        self, cmd_name: str, cmd: GorgonzolaCommand
    ) -> T.Tuple[str, str, str, str]:
        recv_var = cmd.inputs_const[0].name
        recv_type_snake = to_snake_case(cmd.inputs_const[0].type.name)

        if cmd.target:
            method_name = cmd.target
        else:
            method_name = to_snake_case(cmd_name)
            api_prefix = f'{self.api_name.snake.lower()}_'
            method_name = method_name.removeprefix(api_prefix)
            method_name = method_name.removeprefix(f'{recv_type_snake}_')

        conversions = ''
        call_args = []
        for p in cmd.inputs_const[1:]:
            if struct_has_api_objects(p.type, self.api_data):
                conversions += f'        let rust_{p.name} = {p.name}.to_rust();\n'
                call_args.append(f'&rust_{p.name}')
            else:
                call_args.append(p.name)

        args = ', '.join(call_args)
        return recv_var, method_name, args, conversions

    def _generate_object_creation_body(self, cmd_name: str, cmd: GorgonzolaCommand) -> str:
        dispatchable, method_name, args, conversions = self._resolve_method_invocation(
            cmd_name, cmd
        )
        out_var = cmd.inputs_mut[0].name
        return INVOKE_DISPATCHABLE.format(
            conversions=conversions,
            dispatchable=dispatchable,
            method_name=method_name,
            args=args,
            out_var=out_var,
        )

    def _generate_method_call_body(self, cmd_name: str, cmd: GorgonzolaCommand) -> str:
        recv_var, method_name, args, conversions = self._resolve_method_invocation(cmd_name, cmd)
        ret_type = to_rust_ffi_type(cmd.ret, self.api_name.snake)
        if len(cmd.inputs_mut) >= 1:
            out_var = cmd.inputs_mut[0].name
            if ret_type == '()':
                return INVOKE_METHOD_COPY_OUT_VAR.format(
                    conversions=conversions,
                    recv_var=recv_var,
                    method_name=method_name,
                    args=args,
                    out_var=out_var,
                )
            return INVOKE_METHOD_WITH_OUT_VAR.format(
                conversions=conversions,
                recv_var=recv_var,
                method_name=method_name,
                args=args,
                out_var=out_var,
            )
        else:
            return INVOKE_METHOD.format(
                conversions=conversions, recv_var=recv_var, method_name=method_name, args=args
            )

    def _emit_struct_with_api_objects(self, s: GorgonzolaStruct) -> None:
        ffi_config = self.api_data.ffi_config
        stype_header = ffi_config.get('structure_type_header') or f'{self.api_name.pascal}StructureTypeHeader'
        ffi_name = to_snake_case(s.name)
        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('#[repr(C)]')
        self.code_writer.write_line('#[derive(Copy, Clone)]')
        self.code_writer.write_line(f'pub struct {ffi_name} {{')
        self.code_writer.set_indent_multiple(1)
        if isinstance(s, GorgonzolaExtensibleStruct):
            self.code_writer.write_line(f'pub header: {stype_header},')
        for m in s.members:
            field_type = to_ffi_struct_field(m, self.api_data)
            self.code_writer.write_line(f'pub {m.name}: {field_type},')
        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        to_rust_conversion(s, self.api_data, self.code_writer)
        self.code_writer.write_line()

    def _emit_imports(self) -> None:
        ffi_config = self.api_data.ffi_config
        crate_name = ffi_config.get('crate_name', self.api_name.snake.lower())
        custom_imports = ffi_config.get('custom_imports', [])
        exclude_imports = set(ffi_config.get('exclude_imports', []))

        types_in_aspect = [
            t for t in self.api_data.types(self.ASPECT_MASK)
            if isinstance(
                t,
                (
                    GorgonzolaApiObject,
                    GorgonzolaBaseEnum,
                    GorgonzolaStruct,
                ),
            )
            and t.name not in exclude_imports
        ]

        all_imports = list(custom_imports)
        for t in types_in_aspect:
            all_imports.append(f'use {crate_name}::{t.name};')

        stype_header = ffi_config.get('structure_type_header') or f'{self.api_name.pascal}StructureTypeHeader'
        stype_enum = ffi_config.get('structure_type_enum') or f'{self.api_name.pascal}StructureType'
        has_extensible_structs = any(
            isinstance(t, GorgonzolaExtensibleStruct) for t in types_in_aspect
        )
        if has_extensible_structs:
            all_imports.append(f'use {crate_name}::{stype_enum};')
            all_imports.append(f'use {crate_name}::{stype_header};')
        elif any(
            isinstance(s, GorgonzolaStruct) and struct_has_api_objects(s, self.api_data)
            for s in types_in_aspect
        ):
            all_imports.append(f'use {crate_name}::{stype_header};')

        for imp in sorted(all_imports):
            self.code_writer.write_line(imp)

    def _emit_header(self) -> None:
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_line()
        self.code_writer.write_raw(COMMON_RUST_IMPORTS.strip())
        self.code_writer.write_line()
        self._emit_imports()
        self.code_writer.write_line()
        self.code_writer.write_line('const NO_ERROR: i32 = 0;')

    def _emit_internal(self) -> None:
        self._emit_header()
        self.code_writer.write_line()
        self.code_writer.write_raw(RETURN_RESULT_FN.format().strip())
        self.code_writer.write_line()
        self.code_writer.write_raw(RETURN_ON_ERROR_MACRO.strip())
        self.code_writer.write_line()

        for t in self.api_data.types(self.ASPECT_MASK):
            self.dispatch_type(t)
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            self.dispatch_command(cmd)
