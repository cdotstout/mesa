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


class EncodeGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.WIRE

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)
        self.req_arms: T.List[T.Tuple[str, str]] = []
        self.resp_arms: T.List[T.Tuple[str, str]] = []
        self.ext_arms: T.List[T.Tuple[str, str]] = []

    def _emit_header_struct_fields(
        self, s: GorgonzolaStruct, src_expr: str, is_command: bool = False
    ) -> None:
        w = self.code_writer
        if is_command or isinstance(s, GorgonzolaExtensibleStruct):
            w.write_line(f'header: {src_expr}.header,')
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
                        w.write_line(f'{e.name}: {m.type.name}Header {{')
                        with w.indent():
                            self._emit_header_struct_fields(
                                m.type, f'{src_expr}.{e.name}', is_command=False
                            )
                        w.write_line('},')
                        continue
                w.write_line(f'{e.name}: {src_expr}.{e.name},')

    def _emit_dynamic_array_writes(self, s: GorgonzolaStruct, src_expr: str) -> None:
        w = self.code_writer
        for m in s.members:
            if isinstance(m.type, GorgonzolaDynamicArray):
                slice_expr = f'{src_expr}.{m.name}.as_slice()'
                line = f'self.write_bytes(zerocopy::IntoBytes::as_bytes({slice_expr}))?;'
                if w.indent_level * w.INDENT_SIZE + len(line) <= w.MAX_LINE_LENGTH:
                    w.write_line(line)
                else:
                    w.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(')
                    with w.indent():
                        w.write_line(f'{slice_expr},')
                    w.write_line('))?;')
            elif isinstance(m.type, GorgonzolaStruct) and m.type.has_dynamic_array:
                self._emit_dynamic_array_writes(m.type, f'{src_expr}.{m.name}')

    def _emit_encode_with_p_next(
        self,
        var_name: str,
        headers: T.Sequence[T.Tuple[str, str]],
        struct_obj: T.Optional[GorgonzolaStruct] = None,
        is_command: bool = False,
    ) -> None:
        w = self.code_writer
        if struct_obj is not None and struct_obj.has_dynamic_array:
            w.write_line(f'let mut {var_name} = {struct_obj.name}Header {{')
            with w.indent():
                self._emit_header_struct_fields(struct_obj, 'msg', is_command=is_command)
            w.write_line('};')
        else:
            w.write_line(f'let mut {var_name} = *msg;')
        for p_next_var, header_path in headers:
            w.write_line(f'let {p_next_var} = {header_path}.p_next as usize as u64;')
            w.write_line(f'{header_path}.p_next = if {p_next_var} != 0 {{ 1 }} else {{ 0 }};')
        w.write_line(f'self.write_bytes(zerocopy::IntoBytes::as_bytes(&{var_name}))?;')
        if struct_obj is not None and struct_obj.has_dynamic_array:
            self._emit_dynamic_array_writes(struct_obj, 'msg')
        for p_next_var, _ in headers:
            w.write_line(f'if {p_next_var} != 0 {{')
            with w.indent():
                w.write_line(f'self.encode_extensible_chain({p_next_var})?;')
            w.write_line('}')
        w.write_line('Ok(())')

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        api = self.api_name.pascal
        assert obj.req_struct is not None
        cmd_pascal = obj.req_struct.name
        cmd_snake = to_snake_case(cmd_pascal)
        w = self.code_writer

        ext_members = [
            m for m in obj.req_struct.members
            if isinstance(m.type, GorgonzolaExtensibleStruct)
        ]

        w.write_fn(
            f'encode_{cmd_snake}',
            ['&mut self', f'msg: &{cmd_pascal}'],
            ret='std::io::Result<()>',
        )
        with w.indent():
            if ext_members or obj.req_struct.has_dynamic_array:
                headers = [(f'p_next_{m.name}', f'cmd.{m.name}.header') for m in ext_members]
                self._emit_encode_with_p_next(
                    'cmd', headers, struct_obj=obj.req_struct, is_command=True
                )
            else:
                w.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))')
        w.write_line('}')

        self.req_arms.append(
            (f'{api}Protocol::{cmd_pascal}(msg)', f'self.encode_{cmd_snake}(msg)')
        )

        if obj.resp_struct:
            resp_snake = f'resp_{cmd_snake}'
            resp_variant = obj.resp_struct.name
            w.write_fn(
                f'encode_{resp_snake}',
                ['&mut self', f'msg: &{resp_variant}'],
                ret='std::io::Result<()>',
            )
            with w.indent():
                if obj.resp_struct.has_dynamic_array:
                    self._emit_encode_with_p_next(
                        'cmd', [], struct_obj=obj.resp_struct, is_command=True
                    )
                else:
                    w.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))')
            w.write_line('}')

            self.resp_arms.append(
                (f'{api}Protocol::{resp_variant}(msg)', f'self.encode_{resp_snake}(msg)')
            )

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        api = self.api_name.pascal
        variant = normalize_command_name(obj.name, api)
        struct_snake = to_snake_case(variant)
        w = self.code_writer

        w.write_fn(
            f'encode_{struct_snake}',
            ['&mut self', f'msg: &{obj.name}'],
            ret='std::io::Result<()>',
        )
        with w.indent():
            self._emit_encode_with_p_next(
                'obj', [('p_next', 'obj.header')], struct_obj=obj, is_command=False
            )
        w.write_line('}')

        self.ext_arms.append(
            (f'{api}ExtensibleStruct::{variant}(msg)', f'self.encode_{struct_snake}(msg)')
        )

    def _emit_extensible_chain(self) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        w = self.code_writer
        w.write_fn(
            'encode_extensible_chain',
            ['&mut self', 'mut curr: u64'],
            ret='std::io::Result<()>',
        )
        with w.indent():
            w.write_line('while curr != 0 {')
            with w.indent():
                w.write_line(f'let header = unsafe {{ &*(curr as *const {api}StructureTypeHeader) }};')
                w.write_line('let next = header.p_next as usize as u64;')
                w.write_line('match header.stype {')
                with w.indent():
                    for ext in self.api_data.extensible_structs_for_aspect(self.ASPECT_MASK):
                        stype_const = f'{api_upper}_STRUCTURE_TYPE_{to_snake_case(ext.stype).upper()}'
                        w.write_line(f'{stype_const} => {{')
                        with w.indent():
                            if ext.has_dynamic_array:
                                w.write_line(f'let ext = unsafe {{ &*(curr as *const {ext.name}) }};')
                                w.write_line(f'let mut hdr = {ext.name}Header {{')
                                with w.indent():
                                    self._emit_header_struct_fields(ext, 'ext', is_command=False)
                                w.write_line('};')
                                w.write_line('hdr.header.p_next = if next != 0 { 1 } else { 0 };')
                                w.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(&hdr))?;')
                                self._emit_dynamic_array_writes(ext, 'ext')
                            else:
                                w.write_line(f'let mut ext = unsafe {{ *(curr as *const {ext.name}) }};')
                                w.write_line('ext.header.p_next = if next != 0 { 1 } else { 0 };')
                                w.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;')
                        w.write_line('}')
                    w.write_line('_ => {')
                    with w.indent():
                        w.write_line('return Err(std::io::Error::new(')
                        with w.indent():
                            w.write_line('std::io::ErrorKind::InvalidData,')
                            w.write_line('"unknown stype",')
                        w.write_line('))')
                    w.write_line('}')
                w.write_line('}')
                w.write_line('curr = next;')
            w.write_line('}')
            w.write_line('Ok(())')
        w.write_line('}')

    def _emit_encoder_base_methods(self) -> None:
        w = self.code_writer
        w.write_fn('new', ['writer: W'], ret='Encoder<W>')
        with w.indent():
            w.write_line('Encoder {')
            with w.indent():
                w.write_line('writer,')
                w.write_line('bytes_written: 0,')
            w.write_line('}')
        w.write_line('}')
        w.write_line()
        w.write_fn('bytes_written', ['&self'], ret='usize')
        with w.indent():
            w.write_line('self.bytes_written')
        w.write_line('}')
        w.write_line()
        w.write_fn('into_inner', ['self'], ret='W')
        with w.indent():
            w.write_line('self.writer')
        w.write_line('}')
        w.write_line()
        w.write_fn('writer_mut', ['&mut self'], ret='&mut W')
        with w.indent():
            w.write_line('&mut self.writer')
        w.write_line('}')
        w.write_line()
        w.write_fn('writer', ['&self'], ret='&W')
        with w.indent():
            w.write_line('&self.writer')
        w.write_line('}')
        w.write_line()
        w.write_fn('write_bytes', ['&mut self', 'bytes: &[u8]'], ret='std::io::Result<()>', vis='')
        with w.indent():
            w.write_line('self.writer.write_all(bytes)?;')
            w.write_line('self.bytes_written += bytes.len();')
            w.write_line('Ok(())')
        w.write_line('}')

    def _emit_dispatch_fn(
        self, fn_name: str, msg_type: str, arms: T.Sequence[T.Tuple[str, str]]
    ) -> None:
        w = self.code_writer
        w.write_fn(fn_name, ['&mut self', f'msg: &{msg_type}'], ret='std::io::Result<()>')
        with w.indent():
            w.write_line('match msg {')
            with w.indent():
                for pat, expr in arms:
                    w.write_match_arm(pat, expr)
            w.write_line('}')
        w.write_line('}')

    def _emit_internal(self) -> None:
        w = self.code_writer
        self.emit_license()
        self.emit_protocol_imports(self.api_data.encoder_config)
        w.write_line('pub struct Encoder<W> {')
        with w.indent():
            w.write_line('writer: W,')
            w.write_line('bytes_written: usize,')
        w.write_line('}')
        w.write_line()
        w.write_line('impl<W: std::io::Write> Encoder<W> {')
        with w.indent():
            self._emit_encoder_base_methods()

            if self.api_data.extensible_structs:
                self._emit_extensible_chain()

            self.dispatch_all_types()
            self.dispatch_all_commands()

            api = self.api_name.pascal
            self._emit_dispatch_fn(
                'encode_msg', f'{api}Protocol', [*self.req_arms, *self.resp_arms]
            )

            if self.ext_arms:
                w.write_line()
                w.write_line('#[allow(dead_code)]')
                self._emit_dispatch_fn(
                    'encode_extensible_struct', f'{api}ExtensibleStruct', self.ext_arms
                )

        w.write_line('}')
        w.write_line()
        w.write_line("impl<'a> Encoder<&'a mut [u8]> {")
        with w.indent():
            w.write_fn('from_slice', ["buf: &'a mut [u8]"], ret="Encoder<&'a mut [u8]>")
            with w.indent():
                w.write_line('Encoder::new(buf)')
            w.write_line('}')
        w.write_line('}')

