# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import typing as T

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.code_writer import CodeWriter
from gorgonzola.common import (
    ApiAspectMask,
    Direction,
    CommandKind,
    GorgonzolaApiObject,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    GorgonzolaConstant,
    GorgonzolaConstantArray,
    GorgonzolaDynamicArray,
    GorgonzolaEnum,
    GorgonzolaBaseEnum,
    GorgonzolaCommand,
    GorgonzolaStructMember,
    GorgonzolaTypeInstance,
    BaseRustTypeInfo,
    HandleConversion,
    to_rust_protocol_type,
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
fn return_result(result: Result<()>) -> i32 {
    match result {
        Ok(()) => NO_ERROR,
        Err(e) => {
            debug!("error: {:?}", e);
            e.into()
        }
    }
}
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

UNWRAP_OR_ESRCH = 'unwrap_or(-ESRCH)'


def _to_ffi_type_str(type_inst: GorgonzolaTypeInstance, api_data: PostProcessedApiData) -> str:
    match type_inst:
        case GorgonzolaApiObject(name=name):
            return f'*mut {to_prefixed_snake_case(name, api_data.api_name)}'
        case GorgonzolaStruct(name=name) if struct_has_api_objects(type_inst):
            return to_snake_case(name)
        case GorgonzolaStruct(name=name):
            return name
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            return f'[{_to_ffi_type_str(elem, api_data)}; {size}]'
        case GorgonzolaDynamicArray(element_type=elem):
            return f'*mut {_to_ffi_type_str(elem, api_data)}'
        case _:
            return to_rust_protocol_type(type_inst)


def to_ffi_struct_field(member: GorgonzolaStructMember, api_data: PostProcessedApiData) -> str:
    if member.is_pointer:
        return f'*mut {_to_ffi_type_str(member.type, api_data)}'
    return _to_ffi_type_str(member.type, api_data)


def _emit_member_conversion(
    m: GorgonzolaStructMember,
    struct_obj: GorgonzolaStruct,
    handle_conversions: T.Mapping[str, HandleConversion],
    w: CodeWriter,
) -> None:
    match m.type:
        case GorgonzolaApiObject(name=obj_name) if obj_name in handle_conversions:
            conv = handle_conversions[obj_name]
            conv_expr = conv.resolve_expr(f'self.{m.name}')
            w.write_line(f'let {m.name} = if !self.{m.name}.is_null() {{')
            with w.indent():
                w.write_line(f'{conv_expr}')
            w.write_line('} else {')
            with w.indent():
                w.write_line(f'{conv.default}')
            w.write_line('};')
        case GorgonzolaApiObject():
            w.write_line(f'let {m.name} = 0;')
        case GorgonzolaConstantArray(
            element_type=GorgonzolaApiObject(name=elem_name), array_size_name=size_name
        ) if elem_name in handle_conversions:
            conv = handle_conversions[elem_name]
            conv_expr = conv.resolve_expr('ptr')
            count_field = f'num_{m.name}'
            has_count = any(other.name == count_field for other in struct_obj.members)
            limit_expr = (
                f'(self.{count_field} as usize).min({size_name})'
                if has_count
                else f'{size_name} as usize'
            )
            if conv.default == '0':
                w.write_line(f'let mut {m.name} = [0u32; {size_name}];')
            else:
                w.write_line(
                    f'let mut {m.name}: [{conv.rust_type}; {size_name}] = Default::default();'
                )
            w.write_line(f'let limit_{m.name} = {limit_expr};')
            w.write_line(f'for i in 0..limit_{m.name} {{')
            with w.indent():
                w.write_line(f'let ptr = self.{m.name}[i];')
                w.write_line('if !ptr.is_null() {')
                with w.indent():
                    w.write_line(f'{m.name}[i] = {conv_expr};')
                w.write_line('}')
            w.write_line('}')
        case GorgonzolaConstantArray(
            element_type=GorgonzolaApiObject(), array_size_name=size_name
        ):
            w.write_line(f'let {m.name} = [0u32; {size_name}];')
        case GorgonzolaDynamicArray(
            element_type=GorgonzolaApiObject(name=elem_name), size_member_name=size_name
        ) if elem_name in handle_conversions:
            conv = handle_conversions[elem_name]
            conv_expr = conv.resolve_expr('ptr')
            w.write_line(f'let limit_{m.name} = self.{size_name} as usize;')
            w.write_line(
                f'let mut {m.name}: Vec<{conv.rust_type}> = Vec::with_capacity(limit_{m.name});'
            )
            w.write_line(f'if !self.{m.name}.is_null() && limit_{m.name} > 0 {{')
            with w.indent():
                w.write_line(f'let slice = unsafe {{ from_raw_parts(self.{m.name}, limit_{m.name}) }};')
                w.write_line('for &ptr in slice {')
                with w.indent():
                    w.write_line('let val = if !ptr.is_null() {')
                    with w.indent():
                        w.write_line(f'{conv_expr}')
                    w.write_line('} else {')
                    with w.indent():
                        w.write_line(f'{conv.default}')
                    w.write_line('};')
                    w.write_line(f'{m.name}.push(val);')
                w.write_line('}')
            w.write_line(f'}} else if limit_{m.name} > 0 {{')
            with w.indent():
                w.write_line(f'{m.name}.resize(limit_{m.name}, {conv.default});')
            w.write_line('}')
        case GorgonzolaDynamicArray(
            element_type=GorgonzolaApiObject(), size_member_name=size_name
        ):
            w.write_line(f'let {m.name} = vec![0u32; self.{size_name} as usize];')
        case GorgonzolaDynamicArray(size_member_name=size_name):
            w.write_line(f'let limit_{m.name} = self.{size_name} as usize;')
            w.write_line(f'let {m.name} = if !self.{m.name}.is_null() && limit_{m.name} > 0 {{')
            with w.indent():
                w.write_line(
                    f'unsafe {{ from_raw_parts(self.{m.name}, limit_{m.name}) }}.to_vec()'
                )
            w.write_line('} else {')
            with w.indent():
                w.write_line(f'vec![Default::default(); limit_{m.name}]')
            w.write_line('};')
        case _:
            pass


def _emit_rust_struct_literal(
    struct_obj: GorgonzolaStruct, api_data: PostProcessedApiData, w: CodeWriter
) -> None:
    rust_name = struct_obj.name
    w.write_line(f'{rust_name} {{')
    with w.indent():
        stype_header = api_data.ffi_config.structure_type_header
        if isinstance(struct_obj, GorgonzolaExtensibleStruct):
            w.write_line(f'header: {stype_header} {{')
            with w.indent():
                w.write_line('stype: self.header.stype,')
                w.write_line(f'size: std::mem::size_of::<{rust_name}>() as u32,')
                w.write_line('p_next: self.header.p_next as usize as u64,')
            w.write_line('},')

        for m in struct_obj.members:
            match m.type:
                case GorgonzolaApiObject():
                    w.write_line(f'{m.name},')
                case GorgonzolaConstantArray(element_type=GorgonzolaApiObject()):
                    w.write_line(f'{m.name},')
                case GorgonzolaDynamicArray():
                    w.write_line(f'{m.name},')
                case GorgonzolaStruct() if struct_has_api_objects(m.type):
                    w.write_line(f'{m.name}: self.{m.name}.to_rust(),')
                case _:
                    w.write_line(f'{m.name}: self.{m.name},')

        w.write_line('..Default::default()')
    w.write_line('}')


def to_rust_conversion(
    struct_obj: GorgonzolaStruct, api_data: PostProcessedApiData, w: CodeWriter
) -> None:
    rust_name = struct_obj.name
    ffi_name = to_snake_case(rust_name)
    handle_conversions = api_data.ffi_config.handle_conversions

    w.write_line(f'impl {ffi_name} {{')
    with w.indent():
        w.write_fn('to_rust', ['&self'], ret=rust_name)
        with w.indent():
            for m in struct_obj.members:
                _emit_member_conversion(m, struct_obj, handle_conversions, w)
            _emit_rust_struct_literal(struct_obj, api_data, w)
        w.write_line('}')
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


class FFIGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.FFI

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        self.code_writer.write_line(f'pub const {obj.name}: {obj.type_info.name} = {obj.value};')

    def emit_api_object(self, obj: GorgonzolaApiObject) -> None:
        self._emit_type(obj)

    def emit_enum(self, obj: GorgonzolaBaseEnum) -> None:
        self._emit_type(obj)

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        self._emit_type(obj)

    def _emit_type(self, obj: GorgonzolaTypeInstance) -> None:
        if obj.name in self.api_data.ffi_config.exclude_imports:
            return

        if isinstance(obj, GorgonzolaStruct) and struct_has_api_objects(obj):
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
        status_types = self.api_data.ffi_config.status_types

        api_name = self.api_name.snake
        args = [to_rust_ffi_member_arg(p, api_name) for p in obj.inputs]
        ret_type = to_rust_ffi_type(obj.ret, api_name)
        if (
            isinstance(obj.ret, GorgonzolaEnum)
            or obj.ret.name in status_types
            or ret_type in status_types
        ):
            ret_type = 'i32'

        func_name = to_snake_case(obj.name)
        unwrap_expr = 'unwrap_or(())' if ret_type == '()' else UNWRAP_OR_ESRCH

        w = self.code_writer
        w.write_line('#[no_mangle]')
        w.write_fn(func_name, args, ret=ret_type, vis='pub unsafe extern "C"')
        with w.indent():
            w.write_line('catch_unwind(AssertUnwindSafe(|| {')
            with w.indent():
                w.write_line(f'debug!("call: {func_name}");')
                self._emit_ffi_body(obj.name, obj, ret_type)
            w.write_line('}))')
            w.write_line(f'.{unwrap_expr}')
        w.write_line('}')
        w.write_line()

    def _emit_ffi_body(self, cmd_name: str, cmd: GorgonzolaCommand, ret_type: str) -> None:
        match cmd.kind:
            case CommandKind.DESTRUCTOR:
                self._emit_destructor_body(cmd, ret_type)
            case CommandKind.OBJECT_CREATION:
                self._emit_object_creation_body(cmd_name, cmd, ret_type)
            case CommandKind.METHOD_CALL:
                self._emit_method_call_body(cmd_name, cmd, ret_type)

    def _emit_destructor_body(self, cmd: GorgonzolaCommand, ret_type: str) -> None:
        w = self.code_writer
        if cmd.inputs_mut:
            param_var = cmd.inputs_mut[0].name
            w.write_line(f'if !(*{param_var}).is_null() {{')
            with w.indent():
                w.write_line(f'let _ = unsafe {{ Box::from_raw(*{param_var}) }};')
                w.write_line(f'*{param_var} = null_mut();')
            w.write_line('}')
        elif cmd.inputs_const:
            param_var = cmd.inputs_const[0].name
            w.write_line(f'let _ = unsafe {{ Box::from_raw({param_var} as *mut _) }};')
        if ret_type != '()':
            w.write_line('NO_ERROR')

    def _ffi_extensions_for(
        self, s: GorgonzolaTypeInstance
    ) -> T.List[GorgonzolaExtensibleStruct]:
        if not isinstance(s, GorgonzolaExtensibleStruct):
            return []
        exts = [
            ext
            for ext in self.api_data.extensible_structs_for_aspect(self.ASPECT_MASK)
            if ext.extends is s
            or (s.wire_struct is not None and ext.extends is s.wire_struct)
        ]
        if any(struct_has_api_objects(ext) for ext in exts):
            return exts
        return []

    def _emit_method_invocation(
        self, cmd_name: str, cmd: GorgonzolaCommand
    ) -> T.Tuple[str, str, str, T.List[str]]:
        recv_var = cmd.inputs_const[0].name
        recv_type_snake = to_snake_case(cmd.inputs_const[0].type.name)

        if cmd.target:
            method_name = cmd.target
        else:
            method_name = to_snake_case(cmd_name)
            api_prefix = f'{self.api_name.snake.lower()}_'
            method_name = method_name.removeprefix(api_prefix)
            method_name = method_name.removeprefix(f'{recv_type_snake}_')

        call_args = []
        keep_alive_vars: T.List[str] = []
        w = self.code_writer
        for p in cmd.inputs_const[1:]:
            if self._ffi_extensions_for(p.type):
                exts_var = f'rust_{p.name}_exts'
                w.write_line(f'let {exts_var} = {p.name}.convert_p_next_chain();')
                w.write_line(f'let mut rust_{p.name} = {p.name}.to_rust();')
                w.write_line(f'if rust_{p.name}.header.p_next != 0 {{')
                with w.indent():
                    w.write_line(f'rust_{p.name}.header.p_next =')
                    with w.indent():
                        w.write_line(f'{exts_var}.first().map_or(0, |ext| ext.header_ptr());')
                w.write_line('}')
                call_args.append(f'&rust_{p.name}')
                keep_alive_vars.append(exts_var)
            elif struct_has_api_objects(p.type):
                w.write_line(f'let rust_{p.name} = {p.name}.to_rust();')
                call_args.append(f'&rust_{p.name}')
            else:
                call_args.append(p.name)

        return recv_var, method_name, ', '.join(call_args), keep_alive_vars

    def _emit_object_creation_body(
        self, cmd_name: str, cmd: GorgonzolaCommand, ret_type: str
    ) -> None:
        dispatchable, method_name, args, keep_alive = self._emit_method_invocation(
            cmd_name, cmd
        )
        out_var = cmd.inputs_mut[0].name
        w = self.code_writer
        w.write_line(f'let result = {dispatchable}.{method_name}({args});')
        for var in keep_alive:
            w.write_line(f'let _ = &{var};')
        w.write_line('let res = return_on_error!(result);')
        w.write_line(f'*{out_var} = Box::into_raw(Box::new(res)) as _;')
        if ret_type != '()':
            w.write_line('NO_ERROR')

    def _emit_method_call_body(
        self, cmd_name: str, cmd: GorgonzolaCommand, ret_type: str
    ) -> None:
        recv_var, method_name, args, keep_alive = self._emit_method_invocation(
            cmd_name, cmd
        )
        w = self.code_writer
        w.write_line(f'let result = {recv_var}.{method_name}({args});')
        for var in keep_alive:
            w.write_line(f'let _ = &{var};')
        if len(cmd.inputs_mut) >= 1:
            out_var = cmd.inputs_mut[0].name
            if ret_type == '()':
                w.write_line(f'*{out_var} = *result;')
            else:
                w.write_line(f'*{out_var} = return_on_error!(result);')
                w.write_line('NO_ERROR')
        elif ret_type != '()':
            w.write_line('return_result(result)')

    def _emit_p_next_converter(
        self, s: GorgonzolaExtensibleStruct, exts: T.Sequence[GorgonzolaExtensibleStruct]
    ) -> None:
        ffi_name = to_snake_case(s.name)
        ext_enum_name = f'{ffi_name}_ext'
        stype_enum = self.api_data.ffi_config.structure_type_enum
        stype_header = self.api_data.ffi_config.structure_type_header
        w = self.code_writer

        w.write_line(f'enum {ext_enum_name} {{')
        with w.indent():
            for ext in exts:
                w.write_line(f'{ext.stype}({ext.name}),')
        w.write_line('}')
        w.write_line()
        w.write_line(f'impl {ext_enum_name} {{')
        with w.indent():
            w.write_fn('header_ptr', ['&self'], ret='u64', vis='')
            with w.indent():
                w.write_line('match self {')
                with w.indent():
                    for ext in exts:
                        w.write_line(
                            f'Self::{ext.stype}(ext) => &ext.header as *const _ as u64,'
                        )
                w.write_line('}')
            w.write_line('}')
            w.write_line()
            w.write_fn('set_p_next', ['&mut self', 'p_next: u64'], vis='')
            with w.indent():
                w.write_line('match self {')
                with w.indent():
                    for ext in exts:
                        w.write_line(f'Self::{ext.stype}(ext) => ext.header.p_next = p_next,')
                w.write_line('}')
            w.write_line('}')
        w.write_line('}')
        w.write_line()
        w.write_line(f'impl {ffi_name} {{')
        with w.indent():
            w.write_fn('convert_p_next_chain', ['&self'], ret=f'Vec<{ext_enum_name}>', vis='')
            with w.indent():
                w.write_line('let mut exts = Vec::new();')
                w.write_line('let mut curr = self.header.p_next as usize;')
                w.write_line('while curr != 0 {')
                with w.indent():
                    w.write_line(
                        f'let header = unsafe {{ &*(curr as *const {stype_header}) }};'
                    )
                    w.write_line('let next = header.p_next as usize;')
                    for idx, ext in enumerate(exts):
                        ext_ffi = to_snake_case(ext.name)
                        cond = f'header.stype == {stype_enum}::{ext.stype} as u32'
                        prefix = 'if' if idx == 0 else '} else if'
                        w.write_line(f'{prefix} {cond} {{')
                        with w.indent():
                            if struct_has_api_objects(ext):
                                w.write_line(
                                    f'let mut ext = unsafe {{ (*(curr as *const {ext_ffi})).to_rust() }};'
                                )
                            else:
                                w.write_line(
                                    f'let mut ext = unsafe {{ *(curr as *const {ext_ffi}) }};'
                                )
                            w.write_line('ext.header.p_next = 0;')
                            w.write_line(f'exts.push({ext_enum_name}::{ext.stype}(ext));')
                    w.write_line('}')
                    w.write_line('curr = next;')
                w.write_line('}')
                w.write_line('for i in (0..exts.len()).rev() {')
                with w.indent():
                    w.write_line('let next_ptr = if i + 1 < exts.len() {')
                    with w.indent():
                        w.write_line('exts[i + 1].header_ptr()')
                    w.write_line('} else {')
                    with w.indent():
                        w.write_line('0')
                    w.write_line('};')
                    w.write_line('exts[i].set_p_next(next_ptr);')
                w.write_line('}')
                w.write_line('exts')
            w.write_line('}')
        w.write_line('}')
        w.write_line()

    def _emit_struct_with_api_objects(self, s: GorgonzolaStruct) -> None:
        stype_header = self.api_data.ffi_config.structure_type_header
        ffi_name = to_snake_case(s.name)
        w = self.code_writer
        w.write_line('#[repr(C)]')
        w.write_line('#[derive(Copy, Clone)]')
        w.write_line(f'pub struct {ffi_name} {{')
        with w.indent():
            if isinstance(s, GorgonzolaExtensibleStruct):
                w.write_line(f'pub header: {stype_header},')
            for m in s.members:
                field_type = to_ffi_struct_field(m, self.api_data)
                w.write_line(f'pub {m.name}: {field_type},')
        w.write_line('}')
        w.write_line()
        to_rust_conversion(s, self.api_data, w)
        w.write_line()
        ffi_exts = self._ffi_extensions_for(s)
        if isinstance(s, GorgonzolaExtensibleStruct) and ffi_exts:
            self._emit_p_next_converter(s, ffi_exts)

    def _emit_imports(self) -> None:
        ffi_config = self.api_data.ffi_config
        crate_name = ffi_config.crate_name
        custom_imports = ffi_config.custom_imports
        exclude_imports = ffi_config.exclude_imports

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

        stype_header = ffi_config.structure_type_header
        stype_enum = ffi_config.structure_type_enum
        has_extensible_structs = any(
            isinstance(t, GorgonzolaExtensibleStruct) for t in types_in_aspect
        )
        if has_extensible_structs:
            all_imports.append(f'use {crate_name}::{stype_enum};')
            all_imports.append(f'use {crate_name}::{stype_header};')
        elif any(
            isinstance(s, GorgonzolaStruct) and struct_has_api_objects(s)
            for s in types_in_aspect
        ):
            all_imports.append(f'use {crate_name}::{stype_header};')

        for imp in sorted(all_imports):
            self.code_writer.write_line(imp)

    def _emit_header(self) -> None:
        self.emit_license()
        self.code_writer.write_line()
        self.code_writer.write_raw(COMMON_RUST_IMPORTS.strip())
        self.code_writer.write_line()
        self._emit_imports()
        self.code_writer.write_line()
        self.code_writer.write_line('const NO_ERROR: i32 = 0;')

    def _emit_internal(self) -> None:
        self._emit_header()
        self.code_writer.write_line()
        self.code_writer.write_raw(RETURN_RESULT_FN.strip())
        self.code_writer.write_line()
        self.code_writer.write_raw(RETURN_ON_ERROR_MACRO.strip())
        self.code_writer.write_line()

        self.dispatch_all_types()
        self.dispatch_all_commands()
