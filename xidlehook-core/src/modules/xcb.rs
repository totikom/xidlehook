//! Various X-related utilities. The `Xcb` object must be used
//! regardless of whether or not you want to use `NotWhenAudio` - it's
//! xidlehook's simple way to obtain the idle time. The
//! `NotWhenFullscreen` module is used to implement
//! `--not-when-fullscreen` in the example client.

use std::{fmt, rc::Rc, slice, time::Duration};

use log::debug;

use crate::{Module, Progress, Result, TimerInfo};

const WM_STATE: &str = "WM_STATE";
const NET_WM_STATE: &str = "_NET_WM_STATE";
const NET_WM_STATE_FULLSCREEN: &str = "_NET_WM_STATE_FULLSCREEN";
const NET_WM_DESKTOP: &str = "_NET_WM_DESKTOP";
const NET_CURRENT_DESKTOP: &str = "_NET_CURRENT_DESKTOP";
const WM_NAME: &str = "WM_NAME";
const WM_CLASS: &str = "WM_CLASS";

/// See the crate-level documentation
pub struct Xcb {
    conn: xcb::Connection,
    root_window: xcb::Window,
    // Aside from being a property, WM_STATE is also a
    // type that is not present in the xcb bindings.
    type_wm_state:                xcb::Atom,
    atom_net_wm_desktop:          xcb::Atom,
    atom_net_current_desktop:     xcb::Atom,
    atom_net_wm_state:            xcb::Atom,
    atom_net_wm_state_fullscreen: xcb::Atom,
    atom_wm_name:             xcb::Atom,
    atom_wm_class:                xcb::Atom,
}

impl Xcb {
    /// Initialize all the things, like setting up an X connection.
    pub fn new() -> Result<Self> {
        let (conn, _) = xcb::Connection::connect(None)?;

        let setup = conn.get_setup();
        let screen = setup.roots().next().ok_or("no xcb root")?;
        let root_window = screen.root();

        let type_wm_state =
            xcb::xproto::intern_atom(&conn, false, WM_STATE)
                .get_reply()?
                .atom();

        let atom_net_wm_desktop =
            xcb::xproto::intern_atom(&conn, false, NET_WM_DESKTOP)
                .get_reply()?
                .atom();

        let atom_net_current_desktop =
            xcb::xproto::intern_atom(&conn, false, NET_CURRENT_DESKTOP)
                .get_reply()?
                .atom();

        let atom_net_wm_state =
            xcb::xproto::intern_atom(&conn, false, NET_WM_STATE)
                .get_reply()?
                .atom();

        let atom_net_wm_state_fullscreen =
            xcb::xproto::intern_atom(&conn, false, NET_WM_STATE_FULLSCREEN)
                .get_reply()?
                .atom();

        let atom_wm_name =
            xcb::xproto::intern_atom(&conn, false, WM_NAME)
                .get_reply()?
                .atom();

        let atom_wm_class =
            xcb::xproto::intern_atom(&conn, false, WM_CLASS)
                .get_reply()?
                .atom();

        Ok(Self {
            conn,
            root_window,
            type_wm_state,
            atom_net_wm_desktop,
            atom_net_current_desktop,
            atom_net_wm_state,
            atom_net_wm_state_fullscreen,
            atom_wm_name,
            atom_wm_class,
        })
    }
    /// Get the user's idle time using the `XScreenSaver` plugin
    pub fn get_idle(&self) -> Result<Duration> {
        let info = xcb::screensaver::query_info(&self.conn, self.root_window).get_reply()?;
        Ok(Duration::from_millis(info.ms_since_user_input().into()))
    }

    fn query_fullscreen(
        &self,
        root: xcb::Window,
        exceptions_wm_class1: Option<&Vec<String>>,
        exceptions_wm_class2: Option<&Vec<String>>,
        exceptions_wm_name:   Option<&Vec<String>>,
    ) -> Result<bool> {
        let windows = xcb::xproto::query_tree(&self.conn, root).get_reply()?;

        let active_desktop = xcb::xproto::get_property(
            &self.conn,                    // c
            false,                         // delete
            root,                          // window
            self.atom_net_current_desktop, // property
            xcb::xproto::ATOM_ANY,         // type_
            0,                             // long_offset
            u32::MAX,                      // long_length
        )
        .get_reply()?;
        let active_desktop = active_desktop.value();
        let active_desktop = unsafe {
            slice::from_raw_parts(
                active_desktop.as_ptr() as *const xcb::xproto::Atom,
                active_desktop.len()
            )
        };

        for &window in windows.children() {
            let prop_net_wm_state = xcb::xproto::get_property(
                &self.conn,
                false,
                window,
                self.atom_net_wm_state,
                xcb::xproto::ATOM_ATOM,
                0,
                u32::MAX,
            )
            .get_reply()?;

            let prop_wm_state = xcb::xproto::get_property(
                &self.conn,
                false,
                window,
                self.type_wm_state,
                xcb::xproto::ATOM_ANY,
                0,
                u32::MAX,
            )
            .get_reply()?;


            let prop_desktop = xcb::xproto::get_property(
                &self.conn,
                false,
                window,
                self.atom_net_wm_desktop,
                xcb::xproto::ATOM_ANY,
                0,
                u32::MAX,
            )
            .get_reply()?;


            let prop_wm_name = xcb::xproto::get_property(
                &self.conn,
                false,
                window,
                self.atom_wm_name,
                xcb::xproto::ATOM_ANY,
                0,
                u32::MAX,
            )
            .get_reply()?;


            let prop_wm_class = xcb::xproto::get_property(
                &self.conn,
                false,
                window,
                self.atom_wm_class,
                xcb::xproto::ATOM_ANY,
                0,
                u32::MAX,
            )
            .get_reply()?;


            // The safe API can't possibly know what value xcb returned,
            // sadly. Here we are manually transmuting &[c_void] to
            // &[Atom], as we specified we want an atom.
            let value_net_wm_state = prop_net_wm_state.value();
            let value_net_wm_state = unsafe {
                slice::from_raw_parts(
                    value_net_wm_state.as_ptr() as *const xcb::xproto::Atom,
                    value_net_wm_state.len()
                )
            };

            let value_wm_state = prop_wm_state.value();
            let value_wm_state = unsafe {
                slice::from_raw_parts(
                    value_wm_state.as_ptr() as *const u32,
                    value_wm_state.len()
                )
            };

            let value_desktop = prop_desktop.value();
            let value_desktop = unsafe {
                slice::from_raw_parts(
                    value_desktop.as_ptr() as *const xcb::xproto::Atom,
                    value_desktop.len()
                )
            };

            let value_wm_name: &[u8] = prop_wm_name.value();
            let value_wm_name = unsafe {
                std::str::from_utf8_unchecked(
                    slice::from_raw_parts(
                        value_wm_name.as_ptr() as *const u8,
                        value_wm_name.len()
                    )
                )
            };

            let value_wm_class: &[u8] = prop_wm_class.value();
            let value_wm_class = unsafe {
                std::str::from_utf8_unchecked(
                    slice::from_raw_parts(
                        value_wm_class.as_ptr() as *const u8,
                        value_wm_class.len()
                    )
                )
            };
            let value_wm_class: [&str; 2] = value_wm_class.split_once('\0')
                .map(|s| [s.0, s.1.strip_suffix('\0').unwrap_or(s.1)])
                .unwrap_or(["", ""]);

            println!("desktop: {:?}, ad: {:?}, class: {:?}", value_desktop, active_desktop, value_wm_class[0]);
            // println!("wmname: {:?}; wmclass: {:?}", value_wm_name, value_wm_class);
            // println!("wm_state: {:?}; value_desktop: {:?}", value_wm_state, value_desktop);
            // println!("_net_wm_state_fullscreen: {:?}", value_net_wm_state);

            // Window must have _NET_WM_STATE_FULLSCREEN property to
            // be considered as fullscreen AND it must not be Withdrawn.
            if value_net_wm_state
                .iter()
                .any(|&atom| atom == self.atom_net_wm_state_fullscreen)
            && value_wm_state
                .first()
                .map(|&state| state != 0) // 0 is WithdrawnState
                .unwrap_or(false)
            && value_desktop.len() > 0
            && active_desktop.len() > 0
            && value_desktop[0] == active_desktop[0]
            && exceptions_wm_name
                .map(|v| !v.contains(&value_wm_name.to_owned()))
                .unwrap_or(true)
            && exceptions_wm_class1
                .map(|v| !v.contains(&value_wm_class[0].to_owned()))
                .unwrap_or(true)
            && exceptions_wm_class2
                .map(|v| !v.contains(&value_wm_class[1].to_owned()))
                .unwrap_or(true)
            {
                debug!("Window {} was fullscreen", window);
                return Ok(true);
            }

            if self.query_fullscreen(window,
                exceptions_wm_class1,
                exceptions_wm_class2,
                exceptions_wm_name)?
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get whether or not the user's currently active window is
    /// fullscreen
    pub fn get_fullscreen(
        &self,
        exceptions_wm_class1: Option<&Vec<String>>,
        exceptions_wm_class2: Option<&Vec<String>>,
        exceptions_wm_name:   Option<&Vec<String>>,
    ) -> Result<bool> {
        for screen in self.conn.get_setup().roots() {
            if self.query_fullscreen(
                screen.root(),
                exceptions_wm_class1,
                exceptions_wm_class2,
                exceptions_wm_name,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Return a `NotWhenFullscreen` instance for a reference-counted
    /// self
    pub fn not_when_fullscreen(self: Rc<Self>,
            exceptions_wm_class1: Option<Vec<String>>,
            exceptions_wm_class2: Option<Vec<String>>,
            exceptions_wm_name:   Option<Vec<String>>,
    ) -> NotWhenFullscreen {
        NotWhenFullscreen {
            xcb: self,
            exceptions_wm_class1,
            exceptions_wm_class2,
            exceptions_wm_name
        }
    }
}
impl fmt::Debug for Xcb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Xcb")
    }
}

/// See the module-level documentation
pub struct NotWhenFullscreen {
    xcb: Rc<Xcb>,
    exceptions_wm_class1: Option<Vec<String>>,
    exceptions_wm_class2: Option<Vec<String>>,
    exceptions_wm_name:   Option<Vec<String>>,
}
impl Module for NotWhenFullscreen {
    fn pre_timer(&mut self, _timer: TimerInfo) -> Result<Progress> {
        self.xcb.get_fullscreen(
            self.exceptions_wm_class1.as_ref(),
            self.exceptions_wm_class2.as_ref(),
            self.exceptions_wm_name.as_ref()
        ).map(|fullscreen| {
            if fullscreen {
                Progress::Abort
            } else {
                Progress::Continue
            }
        })
    }
}
impl fmt::Debug for NotWhenFullscreen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NotWhenFullscreen")
    }
}
