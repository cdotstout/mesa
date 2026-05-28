# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from contextlib import contextmanager
import os
from pathlib import Path
import typing as T


MAX_LINE_LENGTH: int = 100


class CodeWriter:
    """Helper for emitting indented code and writing it to a file."""

    INDENT_SIZE: int = 4
    MAX_LINE_LENGTH: int = MAX_LINE_LENGTH
    # Width heuristics from rustfmt default configuration:
    # https://github.com/rust-lang/rustfmt/blob/master/src/config/options.rs
    # Under default max_width = 100:
    # - STRUCT_LIT_WIDTH: maximum width for single-line struct literals (18).
    STRUCT_LIT_WIDTH: int = 18

    def __init__(self, path: Path) -> None:
        self.path = path
        self.indent_level: int = 0
        self.lines: T.List[str] = []

    @contextmanager
    def indent(self) -> T.Iterator[None]:
        self.indent_level += 1
        try:
            yield
        finally:
            self.indent_level -= 1

    def write_line(self, line: str = '') -> None:
        if not line:
            self.lines.append('')
        else:
            prefix = ' ' * (self.indent_level * self.INDENT_SIZE)
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
        prefix_len = self.indent_level * self.INDENT_SIZE
        if prefix_len + len(single_line) > self.MAX_LINE_LENGTH and params:
            self.write_line(f'{vis_prefix}fn {name}(')
            with self.indent():
                for p in params:
                    self.write_line(f'{p},')
            self.write_line(f'){ret_suffix} {{')
        else:
            self.write_line(single_line)

    def write_match_arm(self, pattern: str, expr: str) -> None:
        prefix_len = self.indent_level * self.INDENT_SIZE
        single_line = f'{pattern} => {expr},'
        if prefix_len + len(single_line) > self.MAX_LINE_LENGTH:
            self.write_line(f'{pattern} => {{')
            with self.indent():
                self.write_line(expr)
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
            prefix_len = self.indent_level * self.INDENT_SIZE
            if prefix_len + len(single) <= self.MAX_LINE_LENGTH:
                self.write_line(single)
                return
        self.write_line(f'{type_name} {{')
        with self.indent():
            if header_lines:
                for hl in header_lines:
                    self.write_line(hl)
            for f in fields:
                self.write_line(f'{f},')
            if has_default:
                self.write_line('..Default::default()')
        self.write_line('}')

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
