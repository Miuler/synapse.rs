//! ¿Una imagen lleva un diagrama de draw.io? draw.io guarda el diagrama dentro de la imagen que
//! exporta («incluir una copia del diagrama»): en un PNG, un bloque de texto con la clave `mxfile`;
//! en un SVG, el atributo `content` de la etiqueta `<svg>`. Así se puede editar aunque el archivo no
//! se llame `.drawio.png` / `.drawio.svg`.

const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// `true` si el PNG tiene un bloque `tEXt`, `zTXt` o `iTXt` con la clave `mxfile`. Solo mira los
/// bloques anteriores a los datos de la imagen, donde draw.io los escribe.
pub fn png_has_drawio_diagram(bytes: &[u8]) -> bool {
    if !bytes.starts_with(PNG_SIGNATURE) {
        return false;
    }
    let mut pos = PNG_SIGNATURE.len();
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let Some(data_end) = data_start.checked_add(len).filter(|e| *e <= bytes.len()) else {
            return false;
        };
        match kind {
            b"tEXt" | b"zTXt" | b"iTXt" => {
                let data = &bytes[data_start..data_end];
                let key = data.split(|b| *b == 0).next().unwrap_or_default();
                if key == b"mxfile" {
                    return true;
                }
            }
            b"IDAT" | b"IEND" => return false,
            _ => {}
        }
        pos = data_end + 4; // CRC
    }
    false
}

/// `true` si la etiqueta `<svg>` tiene un atributo `content` con un `mxfile` (escapado o no).
pub fn svg_has_drawio_diagram(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(256 * 1024)]);
    let Some(start) = text.find("<svg") else {
        return false;
    };
    let tag = &text[start..text[start..].find('>').map_or(text.len(), |e| start + e)];
    tag.find("content=").is_some_and(|i| {
        let value = &tag[i..];
        value.contains("mxfile") || value.contains("mxGraphModel")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_el_diagrama_en_un_svg() {
        let drawio = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" content="&lt;mxfile host=&quot;app.diagrams.net&quot;&gt;&lt;/mxfile&gt;" width="10"><rect/></svg>"#;
        assert!(svg_has_drawio_diagram(drawio));
        let normal = br#"<svg xmlns="http://www.w3.org/2000/svg"><text>mxfile</text></svg>"#;
        assert!(!svg_has_drawio_diagram(normal));
        assert!(!svg_has_drawio_diagram(b"no es un svg"));
    }

    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0, 0, 0, 0]);
        c
    }

    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = PNG_SIGNATURE.to_vec();
        out.extend(chunk(b"IHDR", &[0; 13]));
        for c in chunks {
            out.extend_from_slice(c);
        }
        out.extend(chunk(b"IDAT", &[1, 2, 3]));
        out.extend(chunk(b"IEND", &[]));
        out
    }

    #[test]
    fn detecta_el_diagrama_incrustado() {
        assert!(png_has_drawio_diagram(&png(&[chunk(b"tEXt", b"mxfile\0%3Cmxfile%3E")])));
        assert!(png_has_drawio_diagram(&png(&[chunk(b"zTXt", b"mxfile\0\0xyz")])));
    }

    #[test]
    fn un_png_normal_no_es_de_drawio() {
        assert!(!png_has_drawio_diagram(&png(&[chunk(b"tEXt", b"Software\0GIMP")])));
        assert!(!png_has_drawio_diagram(&png(&[])));
        assert!(!png_has_drawio_diagram(b"no es un png"));
        // Un bloque con largo falso no hace leer fuera del archivo
        let mut roto = png(&[]);
        roto[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(!png_has_drawio_diagram(&roto));
    }
}
