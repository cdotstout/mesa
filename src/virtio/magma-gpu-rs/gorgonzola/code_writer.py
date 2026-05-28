# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import os
from pathlib import Path
import typing as T


class CodeWriter:
    """Helper for emitting indented code and writing it to a file."""

    INDENT_SIZE: int = 4
    MAX_LINE_LENGTH: int = 100
    # Width heuristics from rustfmt default configuration:
    # https://github.com/rust-lang/rustfmt/blob/master/src/config/options.rs
    # Under default max_width = 100:
    # - STRUCT_LIT_WIDTH: maximum width for single-line struct literals (18).
    # - FN_CALL_WIDTH: maximum width for function call argument lists before vertical wrapping (60).
    STRUCT_LIT_WIDTH: int = 18
    FN_CALL_WIDTH: int = 60

    def __init__(self, path: Path) -> None:
        self.path = path
        self.indent: int = 0
        self.lines: T.List[str] = []

    def set_indent_multiple(self, level: int) -> None:
        self.indent = level

    def write_line(self, line: str = '') -> None:
        if not line:
            self.lines.append('')
        else:
            prefix = ' ' * (self.indent * self.INDENT_SIZE)
            self.lines.append(f'{prefix}{line}')

    def write_lines(self, lines: T.Iterable[str]) -> None:
        for line in lines:
            self.write_line(line)

    def write_raw(self, text: str) -> None:
        for line in text.splitlines():
            self.lines.append(line)

    def write_fn(
        self,
        name: str,
        params: T.Sequence[str] = (),
        ret: str = '',
        vis: str = 'pub',
    ) -> None:
        vis_prefix = f'{vis} ' if vis else ''
        ret_suffix = f' -> {ret}' if ret else ''
        single_params = ', '.join(params)
        single_line = f'{vis_prefix}fn {name}({single_params}){ret_suffix} {{'
        prefix_len = self.indent * self.INDENT_SIZE
        if prefix_len + len(single_line) > self.MAX_LINE_LENGTH and params:
            self.write_line(f'{vis_prefix}fn {name}(')
            self.indent += 1
            for p in params:
                self.write_line(f'{p},')
            self.indent -= 1
            self.write_line(f'){ret_suffix} {{')
        else:
            self.write_line(single_line)

    def write_match_arm(self, pattern: str, expr: str) -> None:
        prefix_len = self.indent * self.INDENT_SIZE
        single_line = f'{pattern} => {expr},'
        if prefix_len + len(single_line) > self.MAX_LINE_LENGTH:
            self.write_line(f'{pattern} => {{')
            self.indent += 1
            self.write_line(expr)
            self.indent -= 1
            self.write_line('}')
        else:
            self.write_line(single_line)

    def write_struct_instantiation(
        self,
        type_name: str,
        fields: T.Sequence[str],
        has_default: bool = False,
        header_lines: T.Optional[T.Sequence[str]] = None,
    ) -> None:
        if not header_lines and not has_default and len(', '.join(fields)) <= self.STRUCT_LIT_WIDTH:
            single = f"{type_name} {{ {', '.join(fields)} }}"
            prefix_len = self.indent * self.INDENT_SIZE
            if prefix_len + len(single) <= self.MAX_LINE_LENGTH:
                self.write_line(single)
                return
        self.write_line(f'{type_name} {{')
        self.indent += 1
        if header_lines:
            for hl in header_lines:
                self.write_line(hl)
        for f in fields:
            self.write_line(f'{f},')
        if has_default:
            self.write_line('..Default::default()')
        self.indent -= 1
        self.write_line('}')

    def write_decoder_match_arm(
        self, pattern: str, variant: str, reader_call: str = 'reader.read_try_obj().ok()?'
    ) -> None:
        prefix_len = self.indent * self.INDENT_SIZE
        inner_call = f'{variant}({reader_call})'
        expr = f'Some({inner_call})'
        single_line = f'{pattern} => {expr},'
        if prefix_len + len(single_line) <= self.MAX_LINE_LENGTH:
            self.write_line(single_line)
        elif len(inner_call) <= self.FN_CALL_WIDTH:
            # rustfmt WidthHeuristics::fn_call_width (default 60): when the call inside
            # Some(...) is <= FN_CALL_WIDTH, it fits horizontally on the next line.
            # Combined with rustfmt's prefer_next_line and match_arm_blocks = true,
            # rustfmt formats the arm as a braced block with a single-line body.
            self.write_line(f'{pattern} => {{')
            self.indent += 1
            self.write_line(expr)
            self.indent -= 1
            self.write_line('}')
        else:
            # When the inner call exceeds FN_CALL_WIDTH, it wraps across lines.
            # rustfmt overflows the call across lines if the call head fits on the line
            # with at least 2 characters remaining (MAX_LINE_LENGTH - 2 = 98) as required
            # by rustfmt's overflow::rewrite_with_parens (used_width + 2).
            call_head = f'{pattern} => Some({variant}('
            if prefix_len + len(call_head) <= self.MAX_LINE_LENGTH - 2:
                self.write_line(call_head)
                self.indent += 1
                self.write_line(f'{reader_call},')
                self.indent -= 1
                self.write_line(')),')
            else:
                self.write_line(f'{pattern} => Some(')
                self.indent += 1
                self.write_line(f'{variant}({reader_call}),')
                self.indent -= 1
                self.write_line('),')

    def write_derive(self, traits: T.Sequence[str]) -> None:
        single = f"#[derive({', '.join(traits)})]"
        prefix_len = self.indent * self.INDENT_SIZE
        if prefix_len + len(single) > self.MAX_LINE_LENGTH:
            self.write_line('#[derive(')
            self.indent += 1
            self.write_line(f"{', '.join(traits)},")
            self.indent -= 1
            self.write_line(')]')
        else:
            self.write_line(single)

    def getvalue(self) -> str:
        return '\n'.join(self.lines)

    def write_file(self) -> None:
        os.makedirs(self.path.parent, exist_ok=True)
        content = self.getvalue()
        if not content.endswith('\n'):
            content += '\n'
        with open(self.path, 'w') as f:
            f.write(content)

    def __str__(self) -> str:
        return self.getvalue()
