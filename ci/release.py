#!/usr/bin/env python3
"""
release.py - Script para automatizar la liberación de versiones en Synapse.rs.

Tareas que realiza:
1. Valida la versión proporcionada por argumento.
2. Actualiza CHANGELOG.md convirtiendo la sección 'Sin publicar' / 'Unreleased' a la nueva versión con la fecha actual.
3. Actualiza la versión en src-tauri/Cargo.toml.
4. Actualiza la versión en package.json.
5. Opcionalmente actualiza src-tauri/tauri.conf.json si contiene version.
6. Realiza `git add`, `git commit` y crea el `git tag`.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path


def run_cmd(cmd: list[str], dry_run: bool = False) -> subprocess.CompletedProcess:
    print(f"-> {' '.join(cmd)}")
    if dry_run:
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")
    return subprocess.run(cmd, check=True, text=True, capture_output=True)


def update_changelog(changelog_path: Path, version: str) -> None:
    if not changelog_path.exists():
        raise FileNotFoundError(f"{changelog_path} no encontrado")

    content = changelog_path.read_text(encoding="utf-8")
    today = datetime.date.today().isoformat()

    # Misma expresión regular que en .github/workflows/release.yml
    heading_re = re.compile(
        r"^##\s+\[?(Unreleased|Sin publicar|"
        r"(?:v)?(?:[0-9]+(?:\.[0-9]+)+(?:-[0-9a-zA-Z.]+)?)|(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
        r")\]?(?:\s*\([^)]*\))?(?:\s+-\s+.*)?\s*$",
        re.MULTILINE | re.IGNORECASE,
    )

    matches = list(heading_re.finditer(content))
    if not matches:
        raise ValueError("No se encontraron encabezados de versión válidos en CHANGELOG.md")

    # Localizar sección sin publicar / unreleased con notas o la primera disponible
    target_match = None
    for i, match in enumerate(matches):
        section_name = match.group(1).lower()
        if section_name in ("unreleased", "sin publicar"):
            end = matches[i + 1].start() if i + 1 < len(matches) else len(content)
            body = content[match.end():end].strip()
            meaningful = [
                line.strip() for line in body.splitlines()
                if line.strip() and not re.fullmatch(r"[-*]\s*", line.strip())
            ]
            if meaningful:
                target_match = match
                break
            if target_match is None:
                target_match = match

    if not target_match:
        raise ValueError(
            "No se encontró la sección '## [Sin publicar]' ni '## [Unreleased]' en CHANGELOG.md"
        )

    # Reemplazamos la cabecera por la versión y fecha, y dejamos una sección sin publicar vacía arriba
    new_header = f"## [Sin publicar] (Unreleased)\n\n## [{version}] - {today}\n\n"
    content = content[: target_match.start()] + new_header + content[target_match.end() :].lstrip("\r\n")

    # Agregar el enlace de la versión al final de CHANGELOG.md
    link_line = f"[{version}]: https://github.com/Miuler/synapse.rs/releases/tag/{version}"
    link_pattern = re.compile(r"^\[([0-9a-zA-Z._-]+)\]:\s*https?://\S+", re.MULTILINE)
    first_link = link_pattern.search(content)

    if first_link:
        # Insertar justo antes del primer enlace existente de versiones
        content = content[: first_link.start()] + f"{link_line}\n" + content[first_link.start() :]
    else:
        # Si no había enlaces previos, añadirlo al final
        content = content.rstrip() + f"\n\n{link_line}\n"

    changelog_path.write_text(content, encoding="utf-8")
    print(f"✓ CHANGELOG.md actualizado con la versión [{version}] - {today} y enlace de release")


def update_cargo_toml(cargo_path: Path, version: str) -> None:
    if not cargo_path.exists():
        raise FileNotFoundError(f"{cargo_path} no encontrado")

    content = cargo_path.read_text(encoding="utf-8")

    # Modificar únicamente version = "..." dentro de la sección [package]
    lines = content.splitlines(keepends=True)
    in_package = False
    updated = False

    for i, line in enumerate(lines):
        trimmed = line.strip()
        if trimmed.startswith("[") and trimmed.endswith("]"):
            in_package = trimmed == "[package]"
            continue

        if in_package and re.match(r'^version\s*=\s*"[^"]*"', trimmed):
            lines[i] = re.sub(r'version\s*=\s*"[^"]*"', f'version = "{version}"', line)
            updated = True
            break

    if not updated:
        raise ValueError(f"No se encontró la clave version en [package] de {cargo_path}")

    cargo_path.write_text("".join(lines), encoding="utf-8")
    print(f"✓ {cargo_path} actualizado a versión {version}")


def update_package_json(package_path: Path, version: str) -> None:
    if not package_path.exists():
        raise FileNotFoundError(f"{package_path} no encontrado")

    content = package_path.read_text(encoding="utf-8")
    # Usar regex para preservar indentación y formato exacto
    new_content, count = re.subn(
        r'("version"\s*:\s*)"[^"]*"',
        rf'\g<1>"{version}"',
        content,
        count=1,
    )

    if count == 0:
        raise ValueError(f"No se encontró la propiedad 'version' en {package_path}")

    package_path.write_text(new_content, encoding="utf-8")
    print(f"✓ {package_path} actualizado a versión {version}")


def update_tauri_conf_json(tauri_conf_path: Path, version: str) -> None:
    if not tauri_conf_path.exists():
        return

    content = tauri_conf_path.read_text(encoding="utf-8")
    new_content, count = re.subn(
        r'("version"\s*:\s*)"[^"]*"',
        rf'\g<1>"{version}"',
        content,
        count=1,
    )
    if count > 0:
        tauri_conf_path.write_text(new_content, encoding="utf-8")
        print(f"✓ {tauri_conf_path} actualizado a versión {version}")


def main():
    parser = argparse.ArgumentParser(
        description="Automatiza el versionado: actualiza CHANGELOG.md, Cargo.toml, package.json, comitea y crea el tag en git."
    )
    parser.add_argument("version", help="Versión a publicar (ejemplo: 2026.10.28.05-alpha o 1.0.0)")
    parser.add_argument(
        "-m",
        "--message",
        help="Mensaje personalizado para el commit y tag (por defecto: 'chore: release <version>')",
        default=None,
    )
    parser.add_argument(
        "--no-tag",
        action="store_true",
        help="Solo modifica archivos y comitea sin crear el tag git.",
    )
    parser.add_argument(
        "--no-commit",
        action="store_true",
        help="Solo modifica los archivos en disco sin realizar commit ni tag.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Muestra qué se haría sin escribir ni ejecutar comandos git que modifiquen el historial.",
    )

    args = parser.parse_args()
    version = args.version.strip()
    tag_name = version  # También soporta versiones sin prefijo v, tal como usa el repo

    root_dir = Path(__file__).resolve().parent.parent
    changelog_path = root_dir / "CHANGELOG.md"
    cargo_path = root_dir / "src-tauri" / "Cargo.toml"
    package_path = root_dir / "package.json"
    tauri_conf_path = root_dir / "src-tauri" / "tauri.conf.json"

    # Verificar que el repositorio git no tenga cambios sin commitear (a menos que sea dry-run)
    if not args.no_commit and not args.dry_run:
        res = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True)
        # Ignorar si el script recién creado u otros untracked están presentes o si hay cambios
        lines = [l for l in res.stdout.splitlines() if not l.endswith("scripts/release.py")]
        # Si hay cambios staged o modificados en archivos de versión, advertir
        for l in lines:
            if "CHANGELOG.md" in l or "Cargo.toml" in l or "package.json" in l:
                sys.exit(f"Error: Hay cambios pendientes en archivos del repositorio:\n{res.stdout}")

    print(f"==> Iniciando proceso de release para versión: {version}")

    if args.dry_run:
        print("[DRY-RUN] Modificaría CHANGELOG.md")
        print("[DRY-RUN] Modificaría src-tauri/Cargo.toml")
        print("[DRY-RUN] Modificaría package.json")
        print(f"[DRY-RUN] Ejecutaría: git add CHANGELOG.md src-tauri/Cargo.toml package.json")
        print(f"[DRY-RUN] Ejecutaría: git commit -m 'chore: release {version}'")
        print(f"[DRY-RUN] Ejecutaría: git tag -a {tag_name} -m 'chore: release {version}'")
        return

    # 1. Modificar archivos
    update_changelog(changelog_path, version)
    update_cargo_toml(cargo_path, version)
    update_package_json(package_path, version)
    update_tauri_conf_json(tauri_conf_path, version)

    if args.no_commit:
        print("✓ Archivos actualizados exitosamente (--no-commit activado).")
        return

    # 2. Git add
    files_to_add = ["CHANGELOG.md", "src-tauri/Cargo.toml", "package.json"]
    if tauri_conf_path.exists():
        files_to_add.append("src-tauri/tauri.conf.json")

    run_cmd(["git", "add"] + files_to_add)

    # 3. Git commit
    commit_msg = args.message if args.message else f"chore: release {version}"
    run_cmd(["git", "commit", "-m", commit_msg])
    print(f"✓ Commit creado: '{commit_msg}'")

    # 4. Git tag
    if not args.no_tag:
        run_cmd(["git", "tag", "-a", tag_name, "-m", commit_msg])
        print(f"✓ Tag creado: '{tag_name}'")

    print(f"\n🎉 ¡Versión {version} preparada exitosamente!")
    print(f"Para subir los cambios y activar el workflow de release ejecuta:")
    print(f"  git push origin main --tags")


if __name__ == "__main__":
    main()
