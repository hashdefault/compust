use crate::{
    rect,
    support::{Desktop, requests::Requests},
};
use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConfigureWindowAux, ConnectionExt as _, GET_GEOMETRY_REQUEST, QUERY_TREE_REQUEST, StackMode,
    },
};

#[test]
fn moves_and_restacks_need_no_tree_or_geometry_queries() -> Result<()> {
    let (mut desktop, mut requests) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Requests::start)?;
    let red = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let blue = desktop.window(rect(60, 60), 0x0000_00ff)?;
    let green = desktop.window(rect(100, 100), 0x0000_ff00)?;
    for window in [red, blue, green] {
        desktop.map(window)?;
    }
    desktop.until_pixel((110, 110), |p| p == [0, 255, 0])?;
    let trees = requests.count(QUERY_TREE_REQUEST);
    let geometries = requests.count(GET_GEOMETRY_REQUEST);

    // Drag one window while the others trade places, as a window manager does.
    for step in 0..=16 {
        desktop
            .conn
            .configure_window(blue, &ConfigureWindowAux::new().x(60 + step))?;
        let restack = if step % 2 == 0 {
            ConfigureWindowAux::new()
                .sibling(green)
                .stack_mode(StackMode::ABOVE)
        } else {
            ConfigureWindowAux::new().stack_mode(StackMode::BELOW)
        };
        desktop.conn.configure_window(red, &restack)?;
    }
    desktop.conn.flush()?;

    // Red ends above green, which stays above blue.
    desktop.until_pixel((110, 110), |p| p == [255, 0, 0])?;
    desktop.until_pixel((140, 140), |p| p == [0, 255, 0])?;
    desktop.until_pixel((90, 150), |p| p == [0, 0, 255])?;
    assert_eq!(
        requests.count(QUERY_TREE_REQUEST) - trees,
        0,
        "moves and restacks queried the window tree"
    );
    assert_eq!(
        requests.count(GET_GEOMETRY_REQUEST) - geometries,
        0,
        "moves queried window geometry"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    requests.finish()
}
