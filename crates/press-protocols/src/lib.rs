// SPDX-License-Identifier: GPL-3.0-or-later
//! Wayland protocols of press's own: what a program that wears a style
//! needs of a desktop and the common protocols have no word for. Each is a
//! file in `protocols/`, read here into the two sides of it: `client` for
//! a program, `server` for a desktop. A desktop that does not offer one is
//! none the worse; the program does without.

/// Putting a window behind the others: `protocols/press-depth-v1.xml`.
pub mod depth {
    #[cfg(feature = "client")]
    pub mod client {
        use wayland_client;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("../../protocols/press-depth-v1.xml");
        }
        use self::__interfaces::*;

        wayland_scanner::generate_client_code!("../../protocols/press-depth-v1.xml");
    }

    #[cfg(feature = "server")]
    pub mod server {
        use wayland_server;
        use wayland_server::protocol::*;

        pub mod __interfaces {
            use wayland_server::protocol::__interfaces::*;
            wayland_scanner::generate_interfaces!("../../protocols/press-depth-v1.xml");
        }
        use self::__interfaces::*;

        wayland_scanner::generate_server_code!("../../protocols/press-depth-v1.xml");
    }
}
