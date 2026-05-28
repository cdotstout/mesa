# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import re


# gl_create_texture -> GlCreateTexture
def to_pascal_case(s: str) -> str:
    return ''.join(word.capitalize() for word in s.split('_'))


# GlCreateTexture -> gl_create_texture, AMD -> amd
def to_snake_case(s: str) -> str:
    s = re.sub(r'(?<=[a-z0-9])([A-Z])', r'_\1', s)
    s = re.sub(r'(?<=[A-Z])([A-Z][a-z])', r'_\1', s)
    return s.lower()


def to_prefixed_snake_case(s: str, api_name: str) -> str:
    snake = to_snake_case(s)
    prefix = api_name.lower() + '_'
    if not snake.startswith(prefix):
        return prefix + snake
    return snake


def create_c_type(s: str, api_name: str) -> str:
    return to_prefixed_snake_case(s, api_name) + '_t'


def normalize_command_name(cmd_name: str, api_name: str) -> str:
    snake = to_snake_case(cmd_name)
    prefix = api_name.lower() + '_'
    snake = snake.removeprefix(prefix)
    return to_pascal_case(snake)


def to_wire_name(name: str, api_name: str) -> str:
    api_pascal = to_pascal_case(api_name)
    if name.startswith(api_pascal):
        return f'{api_pascal}Wire{name[len(api_pascal):]}'
    return f'Wire{name}'

