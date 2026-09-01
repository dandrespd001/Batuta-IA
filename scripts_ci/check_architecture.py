#!/usr/bin/env python3
"""Validate Batuta's local Cargo dependency graph and architectural boundaries."""

from __future__ import annotations

from dataclasses import dataclass
import pathlib
import sys
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEPENDENCY_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    path: str
    anchor: str
    detail: str

    def render(self) -> str:
        return f"[{self.code}] {self.path}#{self.anchor}: {self.detail}"


class Validation:
    def __init__(self, root: pathlib.Path) -> None:
        self.root = root.resolve()
        self.diagnostics: list[Diagnostic] = []
        self.manifests: dict[str, pathlib.Path] = {}
        self.directories: dict[pathlib.Path, str] = {}
        self.edges: dict[str, set[str]] = {}
        self.workspace_dependencies: dict[str, object] = {}

    def add(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.diagnostics.append(Diagnostic(code, path, anchor, detail))

    def relative(self, path: pathlib.Path) -> str:
        try:
            return path.resolve(strict=False).relative_to(self.root).as_posix()
        except ValueError:
            return "."

    def load_toml(self, path: pathlib.Path) -> dict[str, object] | None:
        try:
            value = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
            self.add("ARCHITECTURE_MANIFEST_INVALID", self.relative(path), "toml", str(error))
            return None
        if not isinstance(value, dict):
            self.add("ARCHITECTURE_MANIFEST_INVALID", self.relative(path), "toml", "se esperaba una tabla")
            return None
        return value

    def member_directories(self, workspace: dict[str, object]) -> list[pathlib.Path]:
        section = workspace.get("workspace")
        if not isinstance(section, dict):
            self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "workspace", "falta [workspace]")
            return []
        members = section.get("members")
        if not isinstance(members, list) or not members or not all(isinstance(item, str) for item in members):
            self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "members", "members debe ser una lista no vacía")
            return []
        result: set[pathlib.Path] = set()
        for member in members:
            pure = pathlib.PurePosixPath(member)
            if pure.is_absolute() or "\\" in member or any(part in {"", ".", ".."} for part in pure.parts):
                self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "members", f"ruta no segura: {member}")
                continue
            matches = sorted(self.root.glob(member)) if any(char in member for char in "*?[") else [self.root / member]
            if not matches:
                self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "members", f"miembro no resoluble: {member}")
            for candidate in matches:
                try:
                    resolved = candidate.resolve(strict=True)
                    resolved.relative_to(self.root)
                except (OSError, ValueError):
                    self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "members", f"miembro no resoluble: {member}")
                    continue
                if not (resolved / "Cargo.toml").is_file():
                    self.add("ARCHITECTURE_MANIFEST_INVALID", "Cargo.toml", "members", f"falta Cargo.toml: {member}")
                    continue
                result.add(resolved)
        return sorted(result)

    def discover(self) -> dict[str, dict[str, object]]:
        root_manifest = self.load_toml(self.root / "Cargo.toml")
        if root_manifest is None:
            return {}
        workspace = root_manifest.get("workspace")
        if isinstance(workspace, dict):
            dependencies = workspace.get("dependencies")
            if isinstance(dependencies, dict):
                self.workspace_dependencies = dependencies

        loaded: dict[str, dict[str, object]] = {}
        for directory in self.member_directories(root_manifest):
            manifest = directory / "Cargo.toml"
            value = self.load_toml(manifest)
            if value is None:
                continue
            package = value.get("package")
            name = package.get("name") if isinstance(package, dict) else None
            if not isinstance(name, str) or not name:
                self.add("ARCHITECTURE_MANIFEST_INVALID", self.relative(manifest), "package", "falta package.name")
                continue
            if name in self.manifests:
                self.add("ARCHITECTURE_MANIFEST_INVALID", self.relative(manifest), name, "nombre de crate duplicado")
                continue
            self.manifests[name] = manifest
            self.directories[directory] = name
            loaded[name] = value
        self.edges = {name: set() for name in self.manifests}
        return loaded

    @staticmethod
    def dependency_tables(manifest: dict[str, object]) -> list[dict[str, object]]:
        result: list[dict[str, object]] = []
        for section in DEPENDENCY_SECTIONS:
            value = manifest.get(section)
            if isinstance(value, dict):
                result.append(value)
        targets = manifest.get("target")
        if isinstance(targets, dict):
            for target in targets.values():
                if not isinstance(target, dict):
                    continue
                for section in DEPENDENCY_SECTIONS:
                    value = target.get(section)
                    if isinstance(value, dict):
                        result.append(value)
        return result

    def dependency_target(
        self,
        source: str,
        alias: str,
        specification: object,
    ) -> str | None:
        if not isinstance(specification, dict):
            return None
        actual = specification.get("package", alias)
        if not isinstance(actual, str):
            return None
        source_directory = self.manifests[source].parent
        path_value = specification.get("path")
        if isinstance(path_value, str):
            try:
                target_directory = (source_directory / path_value).resolve(strict=True)
                target_directory.relative_to(self.root)
            except (OSError, ValueError):
                return None
            return self.directories.get(target_directory)
        if specification.get("workspace") is True:
            inherited = self.workspace_dependencies.get(alias)
            if not isinstance(inherited, dict):
                return None
            inherited_actual = inherited.get("package", actual)
            inherited_path = inherited.get("path")
            if not isinstance(inherited_actual, str) or not isinstance(inherited_path, str):
                return None
            try:
                target_directory = (self.root / inherited_path).resolve(strict=True)
                target_directory.relative_to(self.root)
            except (OSError, ValueError):
                return None
            return self.directories.get(target_directory)
        return None

    def build_graph(self, loaded: dict[str, dict[str, object]]) -> None:
        for source, manifest in sorted(loaded.items()):
            for table in self.dependency_tables(manifest):
                for alias, specification in sorted(table.items()):
                    target = self.dependency_target(source, alias, specification)
                    if target is not None:
                        self.edges[source].add(target)

    def strongly_connected_components(self) -> list[tuple[str, ...]]:
        index = 0
        indices: dict[str, int] = {}
        lowlinks: dict[str, int] = {}
        stack: list[str] = []
        on_stack: set[str] = set()
        components: list[tuple[str, ...]] = []

        def visit(node: str) -> None:
            nonlocal index
            indices[node] = index
            lowlinks[node] = index
            index += 1
            stack.append(node)
            on_stack.add(node)
            for target in sorted(self.edges[node]):
                if target not in indices:
                    visit(target)
                    lowlinks[node] = min(lowlinks[node], lowlinks[target])
                elif target in on_stack:
                    lowlinks[node] = min(lowlinks[node], indices[target])
            if lowlinks[node] == indices[node]:
                component: list[str] = []
                while True:
                    member = stack.pop()
                    on_stack.remove(member)
                    component.append(member)
                    if member == node:
                        break
                components.append(tuple(sorted(component)))

        for node in sorted(self.edges):
            if node not in indices:
                visit(node)
        return sorted(components)

    def enforce_boundaries(self) -> None:
        for component in self.strongly_connected_components():
            if len(component) > 1 or (len(component) == 1 and component[0] in self.edges[component[0]]):
                self.add(
                    "ARCHITECTURE_CYCLE",
                    "Cargo.toml",
                    ",".join(component),
                    f"dependencia cíclica local: {', '.join(component)}",
                )
        for target in sorted(self.edges.get("batuta-contract", set())):
            self.add(
                "ARCHITECTURE_CONTRACT_DEPENDENCY",
                self.relative(self.manifests["batuta-contract"]),
                "batuta-contract",
                f"el crate de contrato depende de {target}",
            )
        for source in sorted(self.edges):
            if source != "batuta-cli" and "batuta-cli" in self.edges[source]:
                self.add(
                    "ARCHITECTURE_DOMAIN_TO_CLI",
                    self.relative(self.manifests[source]),
                    source,
                    "el dominio no puede depender de batuta-cli",
                )

    def run(self) -> None:
        loaded = self.discover()
        self.build_graph(loaded)
        self.enforce_boundaries()


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if arguments:
        print(
            Diagnostic(
                "USAGE_INVALID",
                "scripts_ci/check_architecture.py",
                "arguments",
                "este checker no admite argumentos",
            ).render(),
            file=sys.stderr,
        )
        return 2
    if not ROOT.is_dir():
        print(
            Diagnostic("ARCHITECTURE_ROOT_UNREADABLE", ".", "root", "no se puede leer la raíz").render(),
            file=sys.stderr,
        )
        return 2
    validation = Validation(ROOT)
    validation.run()
    diagnostics = sorted(set(validation.diagnostics))
    for diagnostic in diagnostics:
        print(diagnostic.render(), file=sys.stderr)
    if diagnostics:
        return 1
    edges = sum(len(targets) for targets in validation.edges.values())
    print(f"validated {len(validation.manifests)} local crates and {edges} local dependencies")
    return 0


if __name__ == "__main__":
    sys.exit(main())
