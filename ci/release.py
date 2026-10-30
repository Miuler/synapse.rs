#!/usr/bin/env python3
"""
release.py - Script para automatizar y verificar la liberación de versiones en Synapse.rs.

Tareas que realiza:
- En modo local (creación de release):
  1. Valida el formato del tag/versión proporcionado por argumento.
  2. Valida que el tag git no exista previamente en el repositorio.
  3. Valida que la versión no exista ya en CHANGELOG.md (evitando duplicados) ni existan secciones repetidas.
  4. Actualiza CHANGELOG.md convirtiendo la sección 'Sin publicar' / 'Unreleased' a la nueva versión con la fecha actual.
  5. Opcionalmente (--replace-latest / -r), reemplaza la última versión registrada en CHANGELOG.md por la nueva versión,
     actualizando la cabecera, los enlaces y fusionando notas de 'Sin publicar' si las hubiera.
  6. Valida que CHANGELOG.md resultante cumpla las condiciones de integridad.
  7. Actualiza la versión en package.json y tauri.conf.json (no modifica src-tauri/Cargo.toml).
  8. Realiza `git add`, `git commit` (o `--amend`) y crea el `git tag`.

- En modo CI (--generate-notes FILE):
  1. Valida el formato del tag/versión (por argumento o variable de entorno TAG).
  2. Valida que CHANGELOG.md contenga exactamente una sección para la versión y no esté vacía.
  3. Valida que la sección 'Sin publicar' / 'Unreleased' no tenga notas pendientes.
  4. Valida el orden cronológico respecto al release anterior publicado en GitHub.
  5. Genera el archivo con las notas de release (ej: release-notes.md) para `gh release create`.
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Expresiones regulares compartidas con el pipeline de CI
TAG_PATTERN = (
    r"^(?:v)?((?:[0-9]+(?:\.[0-9]+)+(?:-[0-9a-zA-Z.]+)?)|"
    r"(?:(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)))$"
)

HEADING_RE = re.compile(
    r"^##\s+\[?(Unreleased|Sin publicar|"
    r"(?:v)?(?:[0-9]+(?:\.[0-9]+)+(?:-[0-9a-zA-Z.]+)?)|(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r")\]?(?:\s*\([^)]*\))?(?:\s+-\s+.*)?\s*$",
    re.MULTILINE | re.IGNORECASE,
)


def run_cmd(cmd: list[str], dry_run: bool = False) -> subprocess.CompletedProcess:
    print(f"-> {' '.join(cmd)}")
    if dry_run:
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")
    return subprocess.run(cmd, check=True, text=True, capture_output=True)


def parse_changelog_sections(content: str) -> list[tuple[str, str]]:
    matches = list(HEADING_RE.finditer(content))
    if not matches:
        sys.exit("ERROR: No valid version headings found in CHANGELOG.md")

    sections = []
    for i, match in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(content)
        sections.append((match.group(1), content[match.end():end].strip()))
    return sections


def get_latest_version_from_changelog(changelog_path: Path) -> str:
    if not changelog_path.is_file():
        sys.exit("ERROR: CHANGELOG.md does not exist")
    content = changelog_path.read_text(encoding="utf-8")
    sections = parse_changelog_sections(content)
    version_sections = [
        name.lstrip("v") for name, _ in sections
        if name.lower() not in ("unreleased", "sin publicar")
    ]
    if not version_sections:
        sys.exit("ERROR: No se encontró ninguna versión previa en CHANGELOG.md para reemplazar.")
    return version_sections[0]


def validate_changelog_before_release(changelog_path: Path, clean_version: str) -> None:
    if not changelog_path.is_file():
        sys.exit("ERROR: CHANGELOG.md does not exist")

    content = changelog_path.read_text(encoding="utf-8")
    sections = parse_changelog_sections(content)

    version_sections = [
        (name.lstrip("v"), body) for name, body in sections
        if name.lower() not in ("unreleased", "sin publicar")
    ]
    names = [name for name, _ in version_sections]

    # Validar si ya existe sección para la versión a publicar
    if clean_version in names:
        count = names.count(clean_version)
        sys.exit(
            f"ERROR: Expected exactly one CHANGELOG section for {clean_version}; "
            f"found {count} existing section(s) in CHANGELOG.md before release"
        )

    # Validar si existen duplicados en cualquier otra versión
    duplicate_names = sorted(set(name for name in names if names.count(name) > 1))
    if duplicate_names:
        sys.exit(
            f"ERROR: Found duplicate CHANGELOG sections for version(s): {', '.join(duplicate_names)}"
        )


def validate_changelog_after_release(changelog_path: Path, clean_version: str) -> None:
    """Valida CHANGELOG.md con la misma lógica y mensajes exactos esperados en CI."""
    if not changelog_path.is_file():
        sys.exit("ERROR: CHANGELOG.md does not exist")

    content = changelog_path.read_text(encoding="utf-8")
    sections = parse_changelog_sections(content)

    unreleased = [
        body for name, body in sections
        if name.lower() in ("unreleased", "sin publicar")
    ]

    # Falla si Unreleased contiene notas pendientes
    for body in unreleased:
        meaningful = [
            line.strip() for line in body.splitlines()
            if line.strip() and not re.fullmatch(r"[-*]\s*", line.strip())
        ]
        if meaningful:
            sys.exit(
                "ERROR: CHANGELOG.md still has pending Unreleased / Sin publicar notes. "
                "Move them to the version being released or clear the section."
            )

    version_sections = [
        (name.lstrip("v"), body) for name, body in sections
        if name.lower() not in ("unreleased", "sin publicar")
    ]

    names = [name for name, _ in version_sections]
    if names.count(clean_version) != 1:
        sys.exit(
            f"ERROR: Expected exactly one CHANGELOG section for {clean_version}; "
            f"found {names.count(clean_version)}"
        )

    current_index = names.index(clean_version)
    if not version_sections[current_index][1]:
        sys.exit(f"ERROR: CHANGELOG section for {clean_version} is empty")

    duplicate_names = sorted(set(name for name in names if names.count(name) > 1))
    if duplicate_names:
        sys.exit(
            f"ERROR: Found duplicate CHANGELOG sections for version(s): {', '.join(duplicate_names)}"
        )

    print(f"✓ Validación exitosa: exactamente 1 sección para {clean_version} en CHANGELOG.md y con contenido.")


def get_previous_release() -> dict | None:
    try:
        result = subprocess.run(
            [
                "gh", "release", "list",
                "--limit", "100",
                "--json", "tagName,isDraft,isPrerelease,publishedAt",
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        releases = json.loads(result.stdout)
        published = [
            r for r in releases
            if not r.get("isDraft")
            and not r.get("isPrerelease")
            and r.get("publishedAt")
        ]
        return published[0] if published else None
    except Exception as e:
        print(f"Aviso: No se pudo consultar la lista de releases en GitHub ({e}).")
        return None


def generate_release_notes(changelog_path: Path, clean_version: str, tag: str, output_path: Path) -> None:
    validate_changelog_after_release(changelog_path, clean_version)

    content = changelog_path.read_text(encoding="utf-8")
    sections = parse_changelog_sections(content)

    version_sections = [
        (name.lstrip("v"), body) for name, body in sections
        if name.lower() not in ("unreleased", "sin publicar")
    ]
    names = [name for name, _ in version_sections]
    current_index = names.index(clean_version)

    previous = get_previous_release()

    if previous:
        previous_tag = previous["tagName"]
        previous_version = previous_tag.lstrip("v")

        if previous_version in names:
            previous_index = names.index(previous_version)
            if previous_index <= current_index:
                sys.exit(
                    f"ERROR: CHANGELOG order is incorrect: {clean_version} must "
                    f"appear before {previous_version}."
                )
            selected = version_sections[current_index:previous_index]
        else:
            selected = [version_sections[current_index]]
    else:
        # Primer release: incluye todas las secciones versionadas hasta este tag
        selected = version_sections[:current_index + 1]

    notes = []
    for name, body in selected:
        notes.append(f"## {name}\n\n{body}")

    if not notes:
        sys.exit("ERROR: No release notes could be generated")

    output_path.write_text("\n\n---\n\n".join(notes) + "\n", encoding="utf-8")

    print(f"Tag: {tag}")
    print(f"Previous published release: {previous['tagName'] if previous else 'none'}")
    print(f"Versions included: {', '.join(name for name, _ in selected)}")
    print(f"Release notes written to {output_path}")


def update_changelog(changelog_path: Path, version: str) -> None:
    if not changelog_path.exists():
        raise FileNotFoundError(f"{changelog_path} no encontrado")

    content = changelog_path.read_text(encoding="utf-8")
    today = datetime.date.today().isoformat()

    matches = list(HEADING_RE.finditer(content))
    if not matches:
        raise ValueError("No se encontraron encabezados de versión válidos en CHANGELOG.md")

    target_match = None
    target_meaningful = False
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
                target_meaningful = True
                break
            if target_match is None:
                target_match = match

    if not target_match:
        raise ValueError(
            "No se encontró la sección '## [Sin publicar]' ni '## [Unreleased]' en CHANGELOG.md"
        )

    if not target_meaningful:
        sys.exit(
            "ERROR: La sección 'Sin publicar' / 'Unreleased' no contiene notas para publicar."
        )

    new_header = f"## [Sin publicar] (Unreleased)\n\n## [{version}] - {today}\n\n"
    content = content[: target_match.start()] + new_header + content[target_match.end():].lstrip("\r\n")

    link_line = f"[{version}]: https://github.com/Miuler/synapse.rs/releases/tag/{version}"
    link_pattern = re.compile(r"^\[([0-9a-zA-Z._-]+)\]:\s*https?://\S+", re.MULTILINE)
    first_link = link_pattern.search(content)

    if first_link:
        content = content[: first_link.start()] + f"{link_line}\n" + content[first_link.start():]
    else:
        content = content.rstrip() + f"\n\n{link_line}\n"

    changelog_path.write_text(content, encoding="utf-8")
    print(f"✓ CHANGELOG.md actualizado con la versión [{version}] - {today} y enlace de release")


def replace_latest_changelog(changelog_path: Path, new_version: str) -> tuple[str, str]:
    """
    Reemplaza la cabecera y enlace de la última versión en CHANGELOG.md por new_version.
    Si la sección 'Sin publicar' contiene notas, las fusiona con la versión reemplazada.
    """
    if not changelog_path.exists():
        raise FileNotFoundError(f"{changelog_path} no encontrado")

    content = changelog_path.read_text(encoding="utf-8")
    today = datetime.date.today().isoformat()

    matches = list(HEADING_RE.finditer(content))
    if not matches:
        raise ValueError("No se encontraron encabezados de versión válidos en CHANGELOG.md")

    unreleased_idx = None
    target_idx = None
    for i, match in enumerate(matches):
        section_name = match.group(1).lower()
        if section_name in ("unreleased", "sin publicar"):
            if unreleased_idx is None:
                unreleased_idx = i
        else:
            if target_idx is None:
                target_idx = i
                break

    if target_idx is None:
        sys.exit("ERROR: No se encontró ninguna versión previa en CHANGELOG.md para reemplazar.")

    target_match = matches[target_idx]
    old_raw_name = target_match.group(1)
    old_version = old_raw_name.lstrip("v")

    # Verificar si hay notas pendientes en 'Sin publicar' para fusionarlas
    merge_unreleased = False
    unreleased_body_str = ""
    if unreleased_idx is not None:
        unr_match = matches[unreleased_idx]
        next_pos = matches[unreleased_idx + 1].start() if unreleased_idx + 1 < len(matches) else len(content)
        unr_body = content[unr_match.end():next_pos].strip()
        meaningful = [
            line for line in unr_body.splitlines()
            if line.strip() and not re.fullmatch(r"[-*]\s*", line.strip())
        ]
        if meaningful:
            merge_unreleased = True
            unreleased_body_str = unr_body

    if merge_unreleased:
        next_target_pos = matches[target_idx + 1].start() if target_idx + 1 < len(matches) else len(content)
        current_target_body = content[target_match.end():next_target_pos].strip()
        # Las notas pendientes van a la nueva versión; la versión anterior conserva su encabezado y notas
        old_heading = target_match.group(0).rstrip()
        unr_match = matches[unreleased_idx]
        new_block = (
            f"## [Sin publicar] (Unreleased)\n\n"
            f"## [{new_version}] - {today}\n\n"
            f"{unreleased_body_str}\n\n"
            f"{old_heading}\n\n"
            f"{current_target_body}\n\n"
        )
        content = content[:unr_match.start()] + new_block + content[next_target_pos:].lstrip("\r\n")
    else:
        new_header = f"## [{new_version}] - {today}"
        content = content[:target_match.start()] + new_header + content[target_match.end():]

    # Reemplazar o actualizar enlace al final de CHANGELOG.md
    old_link_pattern = re.compile(
        rf"^\[{re.escape(old_version)}\]:\s*https?://\S+", re.MULTILINE
    )
    new_link_line = f"[{new_version}]: https://github.com/Miuler/synapse.rs/releases/tag/{new_version}"
    # Si la versión anterior se conserva (fusión), su enlace no se reemplaza: se añade el nuevo
    if not merge_unreleased and old_link_pattern.search(content):
        content = old_link_pattern.sub(new_link_line, content, count=1)
    else:
        link_pattern = re.compile(r"^\[([0-9a-zA-Z._-]+)\]:\s*https?://\S+", re.MULTILINE)
        first_link = link_pattern.search(content)
        if first_link:
            content = content[: first_link.start()] + f"{new_link_line}\n" + content[first_link.start():]
        else:
            content = content.rstrip() + f"\n\n{new_link_line}\n"

    changelog_path.write_text(content, encoding="utf-8")
    print(f"✓ CHANGELOG.md: reemplazada versión [{old_version}] por [{new_version}] - {today}")
    return old_version, new_version


def update_package_json(package_path: Path, version: str) -> None:
    if not package_path.exists():
        raise FileNotFoundError(f"{package_path} no encontrado")

    content = package_path.read_text(encoding="utf-8")
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
        description="Automatiza y valida el versionado: actualiza o reemplaza versiones en CHANGELOG.md, package.json y genera release notes para CI."
    )
    parser.add_argument(
        "version",
        nargs="?",
        default=None,
        help="Versión o tag a publicar/verificar (ejemplo: 2026.10.28.05-alpha o 1.0.0). Si se omite, se lee de $TAG o $GITHUB_REF_NAME.",
    )
    parser.add_argument(
        "--replace-latest",
        "-r",
        action="store_true",
        help="Reemplaza la última versión registrada en CHANGELOG.md por la nueva versión indicada (en lugar de crear una nueva sección a partir de Sin publicar).",
    )
    parser.add_argument(
        "--generate-notes",
        metavar="FILE",
        help="Modo CI: valida el tag y CHANGELOG.md, y genera el archivo de notas de release en la ruta indicada (ej. release-notes.md).",
    )
    parser.add_argument(
        "-m",
        "--message",
        help="Mensaje personalizado para el commit y tag (por defecto: 'chore: release <version>')",
        default=None,
    )
    parser.add_argument(
        "--amend",
        action="store_true",
        help="Modifica el commit anterior (git commit --amend) en lugar de crear un commit nuevo.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Solo valida el formato de la versión, la unicidad en CHANGELOG.md y que no exista el tag sin modificar archivos.",
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

    raw_version = args.version or os.environ.get("TAG") or os.environ.get("GITHUB_REF_NAME")
    if not raw_version:
        parser.error("Se requiere especificar una versión como argumento o mediante la variable de entorno TAG")

    version = raw_version.strip()

    # 1. Validar patrón de versión
    m_tag = re.fullmatch(TAG_PATTERN, version)
    if not m_tag:
        sys.exit(f"ERROR: Invalid release tag: {version}")

    clean_version = m_tag.group(1).lstrip("v")
    tag_name = version

    root_dir = Path(__file__).resolve().parent.parent
    changelog_path = root_dir / "CHANGELOG.md"
    package_path = root_dir / "package.json"
    tauri_conf_path = root_dir / "src-tauri" / "tauri.conf.json"

    # Si se pide generar notas para CI (--generate-notes)
    if args.generate_notes:
        output_file = Path(args.generate_notes)
        if not output_file.is_absolute():
            output_file = Path.cwd() / output_file
        generate_release_notes(changelog_path, clean_version, tag_name, output_file)
        return

    old_latest_version = None
    if args.replace_latest:
        old_latest_version = get_latest_version_from_changelog(changelog_path)
        content = changelog_path.read_text(encoding="utf-8")
        sections = parse_changelog_sections(content)
        version_names = [
            name.lstrip("v") for name, _ in sections
            if name.lower() not in ("unreleased", "sin publicar")
        ]
        # Asegurar que la nueva versión no colisione con versiones más antiguas
        if clean_version in version_names[1:]:
            sys.exit(
                f"ERROR: Expected exactly one CHANGELOG section for {clean_version}; "
                f"version already exists in an earlier section in CHANGELOG.md."
            )

        # Si no es --no-tag, verificar que el nuevo tag no exista previamente en git (salvo que sea igual a old_latest_version)
        if not args.no_tag and clean_version != old_latest_version:
            for t in {tag_name, clean_version, f"v{clean_version}"}:
                tag_check = subprocess.run(["git", "tag", "-l", t], capture_output=True, text=True)
                if tag_check.stdout.strip():
                    sys.exit(f"ERROR: Git tag '{t}' already exists in the repository")
    else:
        # 2. Validar que el tag git no exista previamente
        if not args.no_tag:
            for t in {tag_name, clean_version, f"v{clean_version}"}:
                tag_check = subprocess.run(["git", "tag", "-l", t], capture_output=True, text=True)
                if tag_check.stdout.strip():
                    sys.exit(f"ERROR: Git tag '{t}' already exists in the repository")

        # 3. Validar estado previo de CHANGELOG.md (evita duplicar la versión y detecta duplicados existentes)
        validate_changelog_before_release(changelog_path, clean_version)

    if args.check:
        mode_desc = f"reemplazo de '{old_latest_version}' por '{version}'" if args.replace_latest else f"nueva versión '{version}'"
        print(f"✓ Validación exitosa: {mode_desc} es válida y consistente con CHANGELOG.md y git.")
        return

    # Verificar que el repositorio git no tenga cambios sin commitear en archivos de versión
    if not args.no_commit and not args.dry_run:
        res = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True)
        lines = [l for l in res.stdout.splitlines() if not l.endswith("ci/release.py")]
        for l in lines:
            if "package.json" in l:
                sys.exit(f"Error: Hay cambios pendientes en archivos de configuración del repositorio:\n{res.stdout}")

    if args.replace_latest:
        print(f"==> Reemplazando última versión '{old_latest_version}' por nueva versión: {version}")
    else:
        print(f"==> Iniciando proceso de release para versión: {version}")

    commit_msg = args.message if args.message else f"chore: release {version}"
    commit_cmd = ["git", "commit", "--amend", "-m", commit_msg] if args.amend else ["git", "commit", "-m", commit_msg]

    if args.dry_run:
        action_msg = f"Reemplazaría última versión [{old_latest_version}] por [{version}] en CHANGELOG.md" if args.replace_latest else "Modificaría CHANGELOG.md"
        print(f"[DRY-RUN] {action_msg}")
        print("[DRY-RUN] Validaría CHANGELOG.md post-edición")
        print(f"[DRY-RUN] Modificaría package.json -> {version}")
        print("[DRY-RUN] Ejecutaría: git add CHANGELOG.md package.json")
        print(f"[DRY-RUN] Ejecutaría: {' '.join(commit_cmd)}")
        if not args.no_tag:
            if args.replace_latest and old_latest_version and old_latest_version != clean_version:
                print(f"[DRY-RUN] Eliminaría tag local anterior '{old_latest_version}' si existe")
            print(f"[DRY-RUN] Ejecutaría: git tag -a {tag_name} -m '{commit_msg}'")
        return

    # 4. Modificar CHANGELOG.md y validar
    if args.replace_latest:
        replace_latest_changelog(changelog_path, version)
    else:
        update_changelog(changelog_path, version)

    validate_changelog_after_release(changelog_path, clean_version)

    # 5. Modificar versiones en archivos de proyecto
    update_package_json(package_path, version)
    update_tauri_conf_json(tauri_conf_path, version)

    if args.no_commit:
        print("✓ Archivos actualizados exitosamente (--no-commit activado).")
        return

    # 6. Git add
    files_to_add = ["CHANGELOG.md", "package.json"]
    if tauri_conf_path.exists():
        files_to_add.append("src-tauri/tauri.conf.json")

    run_cmd(["git", "add"] + files_to_add)

    # 7. Git commit (o git commit --amend si se especificó --amend)
    run_cmd(commit_cmd)
    action_type = "modificado (--amend)" if args.amend else "creado"
    print(f"✓ Commit {action_type}: '{commit_msg}'")

    # 8. Git tag
    if not args.no_tag:
        # En modo reemplazo, si existía el tag local previo y es diferente, eliminarlo
        if args.replace_latest and old_latest_version and old_latest_version != clean_version:
            for old_t in {old_latest_version, f"v{old_latest_version}"}:
                t_check = subprocess.run(["git", "tag", "-l", old_t], capture_output=True, text=True)
                if t_check.stdout.strip():
                    run_cmd(["git", "tag", "-d", old_t])
                    print(f"✓ Tag local previo '{old_t}' eliminado.")

        run_cmd(["git", "tag", "-a", tag_name, "-m", commit_msg])
        print(f"✓ Tag creado: '{tag_name}'")

    print(f"\n🎉 ¡Versión {version} preparada exitosamente!")
    print("Para subir los cambios y activar el workflow de release ejecuta:")
    if args.replace_latest and old_latest_version and old_latest_version != clean_version:
        print(f"  git push origin main --tags")
        print(f"  # Y si ya habías subido el tag anterior al remoto, elimínalo con:")
        print(f"  git push origin :refs/tags/{old_latest_version}")
    else:
        print("  git push origin main --tags")


if __name__ == "__main__":
    main()
