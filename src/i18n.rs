//! The four languages of the WOL•E app, with the app's own words (celular / telemóvel /
//! teléfono, placa de rede / tarjeta de red).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
    PtPt,
    Es,
}

/// `pt_BR.UTF-8`, `pt-PT`, `es_419`, `en-US`… Anything else is English.
pub fn parse(tag: &str) -> Lang {
    let tag = tag.to_ascii_lowercase().replace('_', "-");
    let mut parts = tag.split(['-', '.', '@']);
    match (parts.next(), parts.next()) {
        (Some("pt"), Some("pt")) => Lang::PtPt,
        (Some("pt"), _) => Lang::PtBr,
        (Some("es"), _) => Lang::Es,
        _ => Lang::En,
    }
}

pub fn detect() -> Lang {
    system_locale().map(|l| parse(&l)).unwrap_or(Lang::En)
}

#[cfg(unix)]
fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty() && v != "C" && v != "POSIX" && !v.starts_with("C."))
}

#[cfg(windows)]
fn system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    // SAFETY: the length passed is the buffer's length; the result counts the final NUL.
    let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    (len > 1).then(|| String::from_utf16_lossy(&buffer[..len as usize - 1]))
}

pub enum Msg<'a> {
    Scan,
    Cards,
    Suggested,
    Wired,
    Wireless,
    Virtual,
    Loopback,
    Disconnected,
    NoIpv4,
    WirelessWarning,
    WirelessChosen,
    NoCard,
    CardNotFound(&'a str),
    CardNotUsable(&'a str),
    CardVirtual(&'a str),
    PressEnter,
    LabelName,
    LabelMac,
    LabelIp,
    LabelBroadcast,
    LabelPort,
    Usage,
}

pub fn text(lang: Lang, msg: Msg) -> String {
    use Lang::*;
    use Msg::*;
    match msg {
        Scan => match lang {
            En => "Scan with your phone's camera to add this computer to WOL•E.",
            PtBr => "Aponte a câmera do celular para adicionar este computador ao WOL•E.",
            PtPt => "Aponte a câmara do telemóvel para adicionar este computador ao WOL•E.",
            Es => "Escanea con la cámara del teléfono para agregar esta computadora a WOL•E.",
        }
        .into(),
        Cards => match lang {
            En => "Network cards",
            PtBr | PtPt => "Placas de rede",
            Es => "Tarjetas de red",
        }
        .into(),
        Suggested => match lang {
            En => "suggested",
            PtBr | PtPt | Es => "sugerida",
        }
        .into(),
        Wired => match lang {
            En => "cable",
            PtBr | PtPt => "cabo",
            Es => "cable",
        }
        .into(),
        Wireless => "Wi-Fi".into(),
        Virtual => "virtual".into(),
        Loopback => "loopback".into(),
        Disconnected => match lang {
            En => "disconnected",
            PtBr => "desconectada",
            PtPt => "desligada",
            Es => "desconectada",
        }
        .into(),
        NoIpv4 => match lang {
            En => "no IPv4",
            PtBr | PtPt => "sem IPv4",
            Es => "sin IPv4",
        }
        .into(),
        WirelessWarning => match lang {
            En => "Wake-on-LAN usually works only over a cable, and no wired card is connected. \
                   Showing the Wi-Fi card.",
            PtBr => "O Wake-on-LAN quase sempre só funciona pelo cabo, e nenhuma placa com cabo \
                     está conectada. Mostrando a placa Wi-Fi.",
            PtPt => "O Wake-on-LAN quase sempre só funciona por cabo, e nenhuma placa com cabo \
                     está ligada. A mostrar a placa Wi-Fi.",
            Es => "Wake-on-LAN casi siempre funciona solo por cable, y no hay ninguna tarjeta \
                   con cable conectada. Se muestra la tarjeta Wi-Fi.",
        }
        .into(),
        WirelessChosen => match lang {
            En => "Wake-on-LAN usually works only over a cable: prefer the wired card.",
            PtBr => "O Wake-on-LAN quase sempre só funciona pelo cabo: prefira a placa com cabo.",
            PtPt => "O Wake-on-LAN quase sempre só funciona por cabo: prefira a placa com cabo.",
            Es => "Wake-on-LAN casi siempre funciona solo por cable: prefiere la tarjeta con cable.",
        }
        .into(),
        NoCard => match lang {
            En => "No connected network card with an IPv4 address was found.",
            PtBr => "Nenhuma placa de rede conectada e com endereço IPv4 foi encontrada.",
            PtPt => "Não foi encontrada nenhuma placa de rede ligada e com endereço IPv4.",
            Es => "No se encontró ninguna tarjeta de red conectada y con dirección IPv4.",
        }
        .into(),
        CardNotFound(name) => match lang {
            En => format!("No network card named “{name}”."),
            PtBr | PtPt => format!("Nenhuma placa de rede se chama “{name}”."),
            Es => format!("Ninguna tarjeta de red se llama “{name}”."),
        },
        CardNotUsable(name) => match lang {
            En => format!("“{name}” can't be used: it must be connected, with an IPv4 address and a MAC."),
            PtBr => format!("Não dá para usar “{name}”: ela precisa estar conectada, com endereço IPv4 e MAC."),
            PtPt => format!("Não é possível usar “{name}”: tem de estar ligada, com endereço IPv4 e MAC."),
            Es => format!("No se puede usar “{name}”: debe estar conectada, con dirección IPv4 y MAC."),
        },
        CardVirtual(name) => match lang {
            En => format!("“{name}” is a virtual card: Wake-on-LAN needs the physical one."),
            PtBr => format!("“{name}” é uma placa virtual: o Wake-on-LAN precisa da placa física."),
            PtPt => format!("“{name}” é uma placa virtual: o Wake-on-LAN precisa da placa física."),
            Es => format!("“{name}” es una tarjeta virtual: Wake-on-LAN necesita la tarjeta física."),
        },
        PressEnter => match lang {
            En => "Press Enter to close.",
            PtBr => "Tecle Enter para fechar.",
            PtPt => "Prima Enter para fechar.",
            Es => "Presiona Enter para cerrar.",
        }
        .into(),
        // The labels of the app's Add device form, without the "*" and the hints.
        LabelName => match lang {
            En => "Device Name",
            PtBr | PtPt => "Nome do Dispositivo",
            Es => "Nombre del Dispositivo",
        }
        .into(),
        LabelMac => match lang {
            En => "MAC Address",
            PtBr | PtPt => "Endereço MAC",
            Es => "Dirección MAC",
        }
        .into(),
        LabelIp => "IP".into(),
        LabelBroadcast => match lang {
            En => "Broadcast Address",
            PtBr | PtPt => "Endereço de Broadcast",
            Es => "Dirección de Broadcast",
        }
        .into(),
        LabelPort => match lang {
            En => "UDP Port",
            PtBr | PtPt => "Porta UDP",
            Es => "Puerto UDP",
        }
        .into(),
        Usage => match lang {
            En => "Usage: wole-link [--card NAME] [--text] [--list] [--lang en|pt-BR|pt-PT|es]\n\
                   \n  --card NAME  use this network card instead of the suggested one\
                   \n  --text       QR in the terminal, without a window\
                   \n  --list       list every network card, virtual ones included\
                   \n  --lang       language of the messages",
            PtBr => "Uso: wole-link [--card NOME] [--text] [--list] [--lang en|pt-BR|pt-PT|es]\n\
                     \n  --card NOME  usa esta placa de rede em vez da sugerida\
                     \n  --text       QR no terminal, sem janela\
                     \n  --list       lista todas as placas de rede, também as virtuais\
                     \n  --lang       idioma das mensagens",
            PtPt => "Utilização: wole-link [--card NOME] [--text] [--list] [--lang en|pt-BR|pt-PT|es]\n\
                     \n  --card NOME  usa esta placa de rede em vez da sugerida\
                     \n  --text       QR no terminal, sem janela\
                     \n  --list       lista todas as placas de rede, também as virtuais\
                     \n  --lang       idioma das mensagens",
            Es => "Uso: wole-link [--card NOMBRE] [--text] [--list] [--lang en|pt-BR|pt-PT|es]\n\
                   \n  --card NOMBRE  usa esta tarjeta de red en lugar de la sugerida\
                   \n  --text         QR en la terminal, sin ventana\
                   \n  --list         lista todas las tarjetas de red, también las virtuales\
                   \n  --lang         idioma de los mensajes",
        }
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags() {
        assert_eq!(parse("pt_BR.UTF-8"), Lang::PtBr);
        assert_eq!(parse("pt-PT"), Lang::PtPt);
        assert_eq!(parse("pt_PT@euro"), Lang::PtPt);
        assert_eq!(parse("pt"), Lang::PtBr);
        assert_eq!(parse("es_419"), Lang::Es);
        assert_eq!(parse("es-ES"), Lang::Es);
        assert_eq!(parse("en_US.UTF-8"), Lang::En);
        assert_eq!(parse("de_DE"), Lang::En);
    }
}
