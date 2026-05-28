# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from abc import ABC, abstractmethod
from pathlib import Path

from gorgonzola.code_writer import CodeWriter
from gorgonzola.common import (
    ApiAspectMask,
    CodecConfig,
    CopyrightInfo,
    GorgonzolaApiObject,
    GorgonzolaBaseEnum,
    GorgonzolaCommand,
    GorgonzolaConstant,
    GorgonzolaEnum,
    GorgonzolaExtensibleStruct,
    GorgonzolaFlag,
    GorgonzolaStruct,
    GorgonzolaTypeInstance,
)
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.utils import to_pascal_case, to_snake_case

LICENSE = """\
// Copyright {year} {holder}
// SPDX-License-Identifier: {spdx}
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

"""


class ApiName:
    def __init__(self, api_name: str) -> None:
        self.snake: str = to_snake_case(api_name)
        self.pascal: str = to_pascal_case(api_name)


class BaseGenerator(ABC):
    ASPECT_MASK: ApiAspectMask = ApiAspectMask.NONE

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        self.full_path = full_path
        self.copyright_info = copyright_info
        self.api_name = ApiName(api_name)
        self.api_data = api_data
        self.code_writer = CodeWriter(full_path)

    def emit(self) -> None:
        self._emit_internal()
        self.code_writer.write_file()

    def dispatch_type(self, obj: GorgonzolaTypeInstance) -> None:
        match obj:
            case GorgonzolaConstant():
                self.emit_constant(obj)
            case GorgonzolaApiObject():
                self.emit_api_object(obj)
            case GorgonzolaEnum():
                self.emit_enum(obj)
            case GorgonzolaFlag():
                self.emit_flag(obj)
            case GorgonzolaExtensibleStruct():
                self.emit_extensible_struct(obj)
            case GorgonzolaStruct():
                self.emit_struct(obj)
            case _:
                pass

    def dispatch_command(self, obj: GorgonzolaCommand) -> None:
        self.emit_command(obj)

    def dispatch_all_types(self) -> None:
        for t in self.api_data.types(self.ASPECT_MASK):
            self.dispatch_type(t)

    def dispatch_all_commands(self) -> None:
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            self.dispatch_command(cmd)

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        pass

    def emit_api_object(self, obj: GorgonzolaApiObject) -> None:
        pass

    def emit_enum(self, obj: GorgonzolaBaseEnum) -> None:
        pass

    def emit_flag(self, obj: GorgonzolaFlag) -> None:
        self.emit_enum(obj)

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        pass

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        self.emit_struct(obj)

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        pass

    @abstractmethod
    def _emit_internal(self) -> None:
        pass

    def emit_license(self) -> None:
        c = self.copyright_info
        self.code_writer.write_raw(
            LICENSE.format(year=c.year, holder=c.holder, spdx=c.spdx).rstrip('\n')
        )

    def emit_protocol_imports(self, config: CodecConfig) -> None:
        self.code_writer.write_line(config.protocol_import)
        for imp in config.custom_imports:
            self.code_writer.write_line(imp)
