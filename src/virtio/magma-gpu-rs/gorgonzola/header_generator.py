# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import typing as T

from gorgonzola.common import (
    SINGLE_INDENT,
    MAX_LINE_LENGTH,
    CommonGenerator,
    ApiAspectMask,
    Direction,
    GorgonzolaConstant,
    GorgonzolaEnum,
    GorgonzolaFlag,
    GorgonzolaBaseEnum,
    GorgonzolaApiObject,
    GorgonzolaConstantArray,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    GorgonzolaStructMember,
    GorgonzolaCommand,
    GorgonzolaTypeInstance,
    BaseRustTypeInfo,
)
from gorgonzola.post_process import (
    LayoutEntry,
    U8_BYTE_SIZE,
    U16_BYTE_SIZE,
    U32_BYTE_SIZE,
    U64_BYTE_SIZE,
    PostProcessedApiData,
)
from gorgonzola.utils import (
    create_c_type,
    to_pascal_case,
    to_prefixed_snake_case,
    to_snake_case,
)


def to_c_type(type_inst: GorgonzolaTypeInstance, api_name: str) -> str:
    match type_inst:
        case BaseRustTypeInfo(c_ffi_type=c_type):
            return c_type
        case GorgonzolaBaseEnum(name=name) | GorgonzolaApiObject(name=name):
            return create_c_type(name, api_name)
        case GorgonzolaConstantArray(element_type=elem):
            return to_c_type(elem, api_name) + '*'
        case GorgonzolaStruct(name=name):
            return f'struct {to_snake_case(name)}'
        case _:
            return 'void'


def to_c_arg(
    type_inst: GorgonzolaTypeInstance, name: str, is_const: bool, api_name: str
) -> str:
    match type_inst:
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            elem_c = to_c_type(elem, api_name)
            return f'const {elem_c} {name}[{size}]' if is_const else f'{elem_c} {name}[{size}]'
        case BaseRustTypeInfo() | GorgonzolaBaseEnum() | GorgonzolaApiObject():
            c_type = to_c_type(type_inst, api_name)
            return f'{c_type} {name}' if is_const else f'{c_type}* {name}'
        case _:
            c_type = to_c_type(type_inst, api_name)
            return f'const {c_type}* {name}' if is_const else f'{c_type}* {name}'


def to_c_struct_field(member: GorgonzolaStructMember, api_name: str) -> str:
    if member.is_pointer:
        return f'{to_c_type(member.type, api_name)}* {member.name};'
    match member.type:
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            return f'{to_c_type(elem, api_name)} {member.name}[{size}];'
        case _:
            return f'{to_c_type(member.type, api_name)} {member.name};'


def to_c_constant_def(const: GorgonzolaConstant) -> str:
    return f'#define {const.name} {const.value}\n'


def to_c_api_object_def(obj: GorgonzolaApiObject, api_name: str) -> str:
    snake = to_prefixed_snake_case(obj.name, api_name)
    return f'struct {snake};\ntypedef struct {snake}* {create_c_type(obj.name, api_name)};\n'


def to_c_enum_def(enum: GorgonzolaBaseEnum, api_name: str) -> str:
    prefix = to_snake_case(enum.name).upper()
    lines = [
        f'typedef {enum.type_info.c_ffi_type} {create_c_type(enum.name, api_name)};',
        'enum {',
    ]
    vendor_id_enum = f'{to_pascal_case(api_name)}VendorId'
    for e in enum.entries:
        entry_name = to_snake_case(e.name).upper()
        if isinstance(enum, GorgonzolaFlag):
            width = enum.type_info.num_bytes * 2
            formatted_val = f'0x{e.value:0{width}X}'
        elif enum.name == vendor_id_enum:
            formatted_val = f'0x{e.value:04X}'
        else:
            formatted_val = str(e.value)
        lines.append(f'{SINGLE_INDENT}{prefix}_{entry_name} = {formatted_val},')
    lines.append('};')
    return '\n'.join(lines) + '\n'


def to_c_padding_field(entry: LayoutEntry) -> str:
    if entry.size == U8_BYTE_SIZE:
        return f'uint8_t {entry.name};'
    elif entry.size == U16_BYTE_SIZE:
        return f'uint16_t {entry.name};'
    elif entry.size == U32_BYTE_SIZE:
        return f'uint32_t {entry.name};'
    elif entry.size == U64_BYTE_SIZE:
        return f'uint64_t {entry.name};'
    else:
        return f'uint8_t {entry.name}[{entry.size}];'


def to_c_struct_def(
    struct_obj: GorgonzolaStruct, api_name: str, api_prefix: T.Optional[str] = None
) -> str:
    lines = [f'{to_c_type(struct_obj, api_name)} {{']
    if api_prefix:
        lines.append(f'{SINGLE_INDENT}struct {api_prefix}_structure_type_header header;')
    if struct_obj.ffi_abi_matches_protocol and struct_obj.ffi_layout:
        for entry in struct_obj.ffi_layout.entries:
            if entry.is_padding:
                lines.append(f'{SINGLE_INDENT}{to_c_padding_field(entry)}')
            else:
                member = next((m for m in struct_obj.members if m.name == entry.name), None)
                if member:
                    lines.append(f'{SINGLE_INDENT}{to_c_struct_field(member, api_name)}')
    else:
        for member in struct_obj.members:
            lines.append(f'{SINGLE_INDENT}{to_c_struct_field(member, api_name)}')
    lines.append('};')
    return '\n'.join(lines) + '\n'


def to_c_command_def(cmd: GorgonzolaCommand, api_name: str) -> str:
    args = [
        to_c_arg(m.type, m.name, is_const=(m.direction == Direction.IN), api_name=api_name)
        for m in cmd.inputs
    ]
    prefix = f'{to_c_type(cmd.ret, api_name)} {to_snake_case(cmd.name)}('
    if not args:
        return f'{prefix}void);\n'

    indent = ' ' * len(prefix)
    lines: T.List[str] = []
    current_line = prefix

    for i, arg in enumerate(args):
        suffix = ');' if i == len(args) - 1 else ', '
        item = arg + suffix
        if len(current_line) + len(item) > MAX_LINE_LENGTH and current_line != prefix:
            lines.append(current_line.rstrip())
            current_line = indent + item
        else:
            current_line += item

    lines.append(current_line)
    return '\n'.join(lines) + '\n'


class HeaderGenerator(CommonGenerator):
    ASPECT_MASK = ApiAspectMask.FFI

    def _emit_header_guard_start(self) -> None:
        guard = self.full_path.name.upper().replace('.', '_')
        w = self.code_writer
        w.write_line(f'#ifndef {guard}')
        w.write_line(f'#define {guard}')
        w.write_line()
        w.write_line('#include <stdint.h>')
        w.write_line('#include <stdbool.h>')
        w.write_line('#include <stddef.h>')
        w.write_line()
        w.write_line('#ifdef __cplusplus')
        w.write_line('extern "C" {')
        w.write_line('#endif')

    def _emit_header_guard_end(self) -> None:
        w = self.code_writer
        w.write_line('#ifdef __cplusplus')
        w.write_line('}')
        w.write_line('#endif')
        w.write_line()
        w.write_line('#endif')

    def _emit_constants(self) -> None:
        const_defs = [to_c_constant_def(c) for c in self.api_data.constants(self.ASPECT_MASK)]
        if const_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(const_defs).strip())

    def _emit_api_objects(self) -> None:
        api_name = self.api_name.snake
        obj_defs = [
            to_c_api_object_def(o, api_name) + '\n'
            for o in self.api_data.api_objects(self.ASPECT_MASK)
        ]
        if obj_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(obj_defs).strip())

    def _emit_enums(self) -> None:
        api_name = self.api_name.snake
        enum_defs = [
            to_c_enum_def(e, api_name) + '\n' for e in self.api_data.enums(self.ASPECT_MASK)
        ]
        if enum_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(enum_defs).strip())

    def _emit_flags(self) -> None:
        api_name = self.api_name.snake
        flag_defs = [
            to_c_enum_def(f, api_name) + '\n' for f in self.api_data.flags(self.ASPECT_MASK)
        ]
        if flag_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(flag_defs).strip())

    def _emit_structs(self) -> None:
        api_name = self.api_name.snake
        struct_defs = [
            to_c_struct_def(s, api_name) + '\n' for s in self.api_data.structs(self.ASPECT_MASK)
        ]
        if struct_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(struct_defs).strip())

    def _emit_extensible_structs(self) -> None:
        ext_structs = list(self.api_data.extensible_structs_for_aspect(self.ASPECT_MASK))
        if not ext_structs:
            return
        stype_enum = self.api_data.structure_type_enum_for_aspect(self.ASPECT_MASK)
        if not stype_enum:
            return

        prefix_lower = self.api_name.snake.lower()
        prefix_upper = self.api_name.snake.upper()

        stype_entries = ''
        for entry in stype_enum.entries:
            stype_name = to_snake_case(entry.name).upper()
            stype_entries += (
                f'{SINGLE_INDENT}{prefix_upper}_STRUCTURE_TYPE_{stype_name} = 0x{entry.value:08X},\n'
            )

        header_types = (
            f'typedef enum {{\n{stype_entries}}} {prefix_lower}_structure_type_t;\n\n'
            f'struct {prefix_lower}_structure_type_header {{\n'
            f'{SINGLE_INDENT}{prefix_lower}_structure_type_t s_type;\n'
            f'{SINGLE_INDENT}uint32_t size;\n'
            f'{SINGLE_INDENT}const void* p_next;\n'
            f'}};\n\n'
        )

        ext_defs = ''.join(
            to_c_struct_def(s, self.api_name.snake, prefix_lower) + '\n' for s in ext_structs
        )
        self.code_writer.write_line()
        self.code_writer.write_raw((header_types + ext_defs).strip())

    def _emit_commands(self) -> None:
        api_name = self.api_name.snake
        cmd_defs = [
            to_c_command_def(c, api_name) + '\n'
            for c in self.api_data.commands_for_aspect(self.ASPECT_MASK)
        ]
        if cmd_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(cmd_defs).strip())

    def _emit_internal(self) -> None:
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_line()
        self._emit_header_guard_start()

        self._emit_constants()
        self._emit_api_objects()
        self._emit_enums()
        self._emit_flags()
        self._emit_structs()
        self._emit_extensible_structs()
        self._emit_commands()

        self.code_writer.write_line()
        self._emit_header_guard_end()
