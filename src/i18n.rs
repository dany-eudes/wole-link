//! The four languages of the WOL•E app, with the app's own words (celular / telemóvel /
//! teléfono, placa de rede / tarjeta de red).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
    PtPt,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::En, Lang::PtBr, Lang::PtPt, Lang::Es];

    /// What `--lang` and the settings file take.
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::PtBr => "pt-BR",
            Lang::PtPt => "pt-PT",
            Lang::Es => "es",
        }
    }

    /// Each language in its own words, for the menu.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::PtBr => "Português (Brasil)",
            Lang::PtPt => "Português (Portugal)",
            Lang::Es => "Español",
        }
    }
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
    MenuLanguage,
    MenuAbout,
    MenuSite,
    MenuPlay,
    MenuCoffee,
    MenuContact,
    MenuAddEntry,
    MenuRemoveEntry,
    MenuUpdateEntry,
    AboutVersion(&'a str),
    AboutText,
    AboutLicense,
    AboutFont,
    Close,
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
        MenuLanguage => match lang {
            En => "Language",
            PtBr | PtPt | Es => "Idioma",
        }
        .into(),
        MenuAbout => match lang {
            En => "About WOL•E Link",
            PtBr | PtPt => "Sobre o WOL•E Link",
            Es => "Acerca de WOL•E Link",
        }
        .into(),
        MenuSite => match lang {
            En => "Website and manual",
            PtBr | PtPt => "Site e manual",
            Es => "Sitio web y manual",
        }
        .into(),
        MenuPlay => match lang {
            En => "WOL•E on Google Play",
            PtBr | PtPt => "WOL•E no Google Play",
            Es => "WOL•E en Google Play",
        }
        .into(),
        // The app keeps the service's name in every language.
        MenuCoffee => "Buy Me a Coffee".into(),
        MenuContact => match lang {
            En => "Contact / Support",
            PtBr => "Contato / Suporte",
            PtPt => "Contacto / Suporte",
            Es => "Contacto / Soporte",
        }
        .into(),
        MenuAddEntry => match lang {
            En => "Add to the applications menu",
            PtBr => "Adicionar ao menu de aplicativos",
            PtPt => "Adicionar ao menu de aplicações",
            Es => "Agregar al menú de aplicaciones",
        }
        .into(),
        MenuRemoveEntry => match lang {
            En => "Remove from the applications menu",
            PtBr => "Remover do menu de aplicativos",
            PtPt => "Remover do menu de aplicações",
            Es => "Quitar del menú de aplicaciones",
        }
        .into(),
        MenuUpdateEntry => match lang {
            En => "Update the applications menu (file moved)",
            PtBr => "Atualizar o menu de aplicativos (arquivo movido)",
            PtPt => "Atualizar o menu de aplicações (ficheiro movido)",
            Es => "Actualizar el menú de aplicaciones (archivo movido)",
        }
        .into(),
        AboutVersion(version) => match lang {
            En => format!("Version {version}"),
            PtBr | PtPt => format!("Versão {version}"),
            Es => format!("Versión {version}"),
        },
        AboutText => match lang {
            En => "Shows a QR code that adds this computer to WOL•E, the Wake-on-LAN app for Android.",
            PtBr => "Mostra um QR que adiciona este computador ao WOL•E, o app de Wake-on-LAN para Android.",
            PtPt => "Mostra um código QR que adiciona este computador ao WOL•E, a aplicação de \
                     Wake-on-LAN para Android.",
            Es => "Muestra un código QR que agrega esta computadora a WOL•E, la app de Wake-on-LAN \
                   para Android.",
        }
        .into(),
        AboutLicense => match lang {
            En => "MIT License",
            PtBr | PtPt => "Licença MIT",
            Es => "Licencia MIT",
        }
        .into(),
        AboutFont => match lang {
            En => "Font: Noto Sans, SIL Open Font License 1.1",
            PtBr => "Fonte: Noto Sans, SIL Open Font License 1.1",
            PtPt => "Tipo de letra: Noto Sans, SIL Open Font License 1.1",
            Es => "Fuente: Noto Sans, SIL Open Font License 1.1",
        }
        .into(),
        Close => match lang {
            En => "Close",
            PtBr | PtPt => "Fechar",
            Es => "Cerrar",
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
