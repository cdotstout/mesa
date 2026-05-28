# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations

from pathlib import Path
import typing as T

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.common import (
    ApiAspectMask,
    CopyrightInfo,
    GorgonzolaDynamicArray,
    GorgonzolaExtensibleStruct,
    GorgonzolaCommand,
    GorgonzolaStruct,
)
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.utils import normalize_command_name, to_snake_case


class DecodeGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.WIRE
    FN_CALL_WIDTH: int = 60

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)
        self.req_arms: T.List[T.Tuple[str, str, str]] = []
        self.resp_arms: T.List[T.Tuple[str, str, str]] = []
        self.ext_arms: T.List[T.Tuple[str, str, str]] = []
        self.helper_structs: T.List[T.Tuple[str, GorgonzolaStruct, bool]] = []

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        assert obj.req_struct is not None
        cmd_pascal = obj.req_struct.name
        cmd_snake_lower = to_snake_case(cmd_pascal)
        cmd_snake = cmd_snake_lower.upper()
        opcode_const = f'{api_upper}_OPCODE_{cmd_snake}'
        if obj.req_struct.has_dynamic_array:
            fn_name = f'decode_{cmd_snake_lower}'
            self.helper_structs.append((fn_name, obj.req_struct, True))
            req_reader_call = f'{fn_name}(reader)?'
        else:
            req_reader_call = 'reader.read_try_obj().ok()?'
        self.req_arms.append((opcode_const, f'{api}Protocol::{cmd_pascal}', req_reader_call))

        if obj.resp_struct:
            resp_opcode_const = f'{api_upper}_OPCODE_RESP_{cmd_snake}'
            resp_name = obj.resp_struct.name
            if obj.resp_struct.has_dynamic_array:
                resp_fn_name = f'decode_resp_{cmd_snake_lower}'
                self.helper_structs.append((resp_fn_name, obj.resp_struct, True))
                resp_reader_call = f'{resp_fn_name}(reader)?'
            else:
                resp_reader_call = 'reader.read_try_obj().ok()?'
            self.resp_arms.append(
                (resp_opcode_const, f'{api}Protocol::{resp_name}', resp_reader_call)
            )

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        stype_name = to_snake_case(obj.stype).upper()
        stype_const = f'{api_upper}_STRUCTURE_TYPE_{stype_name}'
        struct_pascal = normalize_command_name(obj.name, api)
        if obj.has_dynamic_array:
            fn_name = f'decode_{to_snake_case(struct_pascal)}'
            self.helper_structs.append((fn_name, obj, False))
            ext_reader_call = f'{fn_name}(reader)?'
        else:
            ext_reader_call = 'reader.read_try_obj().ok()?'
        self.ext_arms.append(
            (stype_const, f'{api}ExtensibleStruct::{struct_pascal}', ext_reader_call)
        )

    def _emit_dynamic_array_reads(
        self, s: GorgonzolaStruct, hdr_expr: str, prefix: str = ''
    ) -> None:
        w = self.code_writer
        for m in s.members:
            if isinstance(m.type, GorgonzolaDynamicArray):
                var_name = f'{prefix}{m.name}'
                count_expr = f'{hdr_expr}.{m.type.size_member_name} as usize'
                line = f'let {var_name} = reader.read_vec({count_expr}).ok()?;'
                if w.indent_level * w.INDENT_SIZE + len(line) <= w.MAX_LINE_LENGTH:
                    w.write_line(line)
                else:
                    w.write_line(f'let {var_name} = reader')
                    with w.indent():
                        w.write_line(f'.read_vec({count_expr})')
                        w.write_line('.ok()?;')
            elif isinstance(m.type, GorgonzolaStruct) and m.type.has_dynamic_array:
                self._emit_dynamic_array_reads(
                    m.type, f'{hdr_expr}.{m.name}', f'{prefix}{m.name}_'
                )

    def _emit_decoded_struct_fields(
        self,
        s: GorgonzolaStruct,
        hdr_expr: str,
        prefix: str = '',
        is_command: bool = False,
    ) -> None:
        w = self.code_writer
        if is_command or isinstance(s, GorgonzolaExtensibleStruct):
            w.write_line(f'header: {hdr_expr}.header,')
        members_by_name = {m.name: m for m in s.members}
        if s.protocol_layout:
            for e in s.protocol_layout.entries:
                if not e.is_padding:
                    m = members_by_name.get(e.name)
                    if (
                        m is not None
                        and isinstance(m.type, GorgonzolaStruct)
                        and m.type.has_dynamic_array
                    ):
                        w.write_line(f'{e.name}: {m.type.name} {{')
                        with w.indent():
                            self._emit_decoded_struct_fields(
                                m.type,
                                f'{hdr_expr}.{e.name}',
                                f'{prefix}{e.name}_',
                                is_command=False,
                            )
                        w.write_line('},')
                        continue
                w.write_line(f'{e.name}: {hdr_expr}.{e.name},')
        for m in s.dynamic_array_members:
            if prefix:
                w.write_line(f'{m.name}: {prefix}{m.name},')
            else:
                w.write_line(f'{m.name},')

    def _emit_helper_fn(
        self, fn_name: str, s: GorgonzolaStruct, is_command: bool
    ) -> None:
        w = self.code_writer
        w.write_line('#[allow(dead_code)]')
        w.write_fn(fn_name, ['reader: &mut Reader'], ret=f'Option<{s.name}>', vis='')
        with w.indent():
            w.write_line(f'let hdr: {s.name}Header = reader.read_try_obj().ok()?;')
            self._emit_dynamic_array_reads(s, 'hdr')
            w.write_line(f'Some({s.name} {{')
            with w.indent():
                self._emit_decoded_struct_fields(s, 'hdr', is_command=is_command)
            w.write_line('})')
        w.write_line('}')

    def _write_decoder_match_arm(
        self, pattern: str, variant: str, reader_call: str = 'reader.read_try_obj().ok()?'
    ) -> None:
        w = self.code_writer
        prefix_len = w.indent_level * w.INDENT_SIZE
        inner_call = f'{variant}({reader_call})'
        expr = f'Some({inner_call})'
        single_line = f'{pattern} => {expr},'
        if prefix_len + len(single_line) <= w.MAX_LINE_LENGTH:
            w.write_line(single_line)
        elif len(inner_call) <= self.FN_CALL_WIDTH:
            # rustfmt WidthHeuristics::fn_call_width (default 60): when the call inside
            # Some(...) is <= FN_CALL_WIDTH, it fits horizontally on the next line.
            # Combined with rustfmt's prefer_next_line and match_arm_blocks = true,
            # rustfmt formats the arm as a braced block with a single-line body.
            w.write_line(f'{pattern} => {{')
            with w.indent():
                w.write_line(expr)
            w.write_line('}')
        else:
            # When the inner call exceeds FN_CALL_WIDTH, it wraps across lines.
            # rustfmt overflows the call across lines if the call head fits on the line
            # with at least 2 characters remaining (MAX_LINE_LENGTH - 2 = 98) as required
            # by rustfmt's overflow::rewrite_with_parens (used_width + 2).
            call_head = f'{pattern} => Some({variant}('
            if prefix_len + len(call_head) <= w.MAX_LINE_LENGTH - 2:
                w.write_line(call_head)
                with w.indent():
                    w.write_line(f'{reader_call},')
                w.write_line(')),')
            else:
                w.write_line(f'{pattern} => Some(')
                with w.indent():
                    w.write_line(f'{variant}({reader_call}),')
                w.write_line('),')

    def _emit_decode_fn(
        self,
        fn_name: str,
        ret_type: str,
        header_type: str,
        field_name: str,
        primary_arms: T.List[T.Tuple[str, str, str]],
        secondary_arms: T.Optional[T.List[T.Tuple[str, str, str]]] = None,
    ) -> None:
        w = self.code_writer
        w.write_fn(fn_name, ['reader: &mut Reader'], ret=f'Option<{ret_type}>')
        with w.indent():
            w.write_line(f'if reader.available_bytes() < std::mem::size_of::<{header_type}>() {{')
            with w.indent():
                w.write_line('return None;')
            w.write_line('}')
            w.write_line(f'let header = reader.peek_obj::<{header_type}>().ok()?;')
            w.write_line(f'match header.{field_name} {{')
            with w.indent():
                for pat, var, reader_call in primary_arms:
                    self._write_decoder_match_arm(pat, var, reader_call)
                if secondary_arms:
                    w.write_line()
                    for pat, var, reader_call in secondary_arms:
                        self._write_decoder_match_arm(pat, var, reader_call)
                w.write_line('_ => None,')
            w.write_line('}')
        w.write_line('}')

    def _emit_internal(self) -> None:
        self.dispatch_all_types()
        self.dispatch_all_commands()

        api = self.api_name.pascal
        self.emit_license()
        self.emit_protocol_imports(self.api_data.decoder_config)
        for fn_name, struct_obj, is_command in self.helper_structs:
            self.code_writer.write_line()
            self._emit_helper_fn(fn_name, struct_obj, is_command)
        self.code_writer.write_line()
        self._emit_decode_fn(
            'decode',
            f'{api}Protocol',
            f'{api}CommandHeader',
            'opcode',
            self.req_arms,
            self.resp_arms,
        )

        if self.ext_arms:
            self.code_writer.write_line()
            self.code_writer.write_line('#[allow(dead_code)]')
            self._emit_decode_fn(
                'decode_extensible_struct',
                f'{api}ExtensibleStruct',
                f'{api}StructureTypeHeader',
                'stype',
                self.ext_arms,
            )

