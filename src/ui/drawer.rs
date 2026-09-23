use std::rc::Rc;

use dioxus::{html::geometry::ClientPoint, prelude::*};

use crate::ui::{
    components::icon::{self, Icon},
    reader::SWIPE_MIN_PX,
};

#[css_module("/src/ui/drawer.css")]
struct Styles;

#[component]
pub(crate) fn Drawer(mut open: Signal<bool>, label: String, children: Element) -> Element {
    let mut drag_start = use_signal(|| None::<ClientPoint>);
    let mut drag_dx = use_signal(|| 0.0);
    let mut drawer_el = use_signal(|| None::<Rc<MountedData>>);
    let mut width = use_signal(|| 0.0);

    rsx! {
        div {
            class: "{Styles::drawer__backdrop}",
            "data-state": if open() { "open" } else { "closed" },
            aria_hidden: true,
            "data-dragging": if drag_dx() > 0.0 { true },
            style: if drag_dx() > 0.0 { "opacity: {backdrop_opacity(drag_dx(), width())}" },
            onclick: move |_| open.set(false),
        }
        aside {
            class: "{Styles::drawer}",
            "data-state": if open() { "open" } else { "closed" },
            inert: if !open() { true },
            aria_label: "{label}",
            "data-dragging": if drag_dx() > 0.0 { true },
            style: if drag_dx() > 0.0 { "transform: translateX({drag_dx}px)" },
            onkeydown: move |e| e.stop_propagation(),
            onmounted: move |e| drawer_el.set(Some(e.data())),
            onpointerdown: move |e| {
                if !drags(&e.pointer_type()) {
                    return;
                }
                drag_start.set(Some(e.client_coordinates()));
                let Some(el) = drawer_el() else {
                    return;
                };
                spawn(async move {
                    if let Ok(rect) = el.get_client_rect().await {
                        width.set(rect.size.width);
                    }
                });
            },
            onpointermove: move |e| {
                let Some(start) = drag_start() else {
                    return;
                };
                let delta = e.client_coordinates() - start;
                drag_dx.set(drag_offset(delta.x, delta.y));
            },
            onpointerup: move |e| {
                drag_dx.set(0.0);
                let Some(start) = drag_start.take() else {
                    return;
                };
                let delta = e.client_coordinates() - start;
                if closes(delta.x as i32, delta.y as i32) {
                    open.set(false);
                }
            },
            onpointercancel: move |_| {
                drag_dx.set(0.0);
                drag_start.set(None);
            },
            div {
                class: "{Styles::drawer__header}",
                span {
                    "{label}"
                }
                button {
                    class: "icon-button",
                    aria_label: "Close {label}",
                    onclick: move |_| open.set(false),
                    Icon {
                        icon: icon::CLOSE,
                    }
                }
            }
            {children}
        }
    }
}

fn drags(pointer_type: &str) -> bool {
    pointer_type != "mouse"
}

fn closes(dx: i32, dy: i32) -> bool {
    dx > 0 && dx.unsigned_abs() >= SWIPE_MIN_PX && dx.unsigned_abs() > dy.unsigned_abs()
}

fn backdrop_opacity(dx: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return 1.0;
    }
    (1.0 - dx / width).clamp(0.0, 1.0)
}

fn drag_offset(dx: f64, dy: f64) -> f64 {
    if dx.abs() > dy.abs() {
        dx.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod test {
    use super::{backdrop_opacity, closes, drag_offset, drags};

    const DRAWER_CSS: &str = include_str!("drawer.css");
    const SLIDE: &str = "0.2s";

    fn rule<'a>(selector: &str) -> &'a str {
        DRAWER_CSS
            .split_once(&format!("{selector} {{"))
            .unwrap_or_else(|| panic!("no rule for {selector}"))
            .1
            .split_once('}')
            .expect("an unclosed rule")
            .0
    }

    #[test]
    fn the_drawer_pays_its_own_safe_area_because_fixed_escapes_the_body_box() {
        assert_eq!(
            DRAWER_CSS.matches("env(safe-area-inset-").count(),
            3,
            "top, right and bottom touch the screen edge; the left edge is over the page",
        );
        assert!(!DRAWER_CSS.contains("safe-aria"));
    }

    #[test]
    fn the_drawer_stays_visible_for_the_whole_slide_out() {
        let closed = rule(".drawer");

        assert!(closed.contains(&format!("transform {SLIDE}")));
        assert!(
            closed.contains(&format!("visibility 0s linear {SLIDE}")),
            "a shorter delay hides the drawer mid-slide; a longer one leaves it \
             hit-testable off-screen",
        );
        assert!(DRAWER_CSS.contains("@media (prefers-reduced-motion: reduce)"));
    }

    #[test]
    fn the_backdrop_fades_with_the_slide_and_stops_catching_taps_once_gone() {
        let closed = rule(".drawer__backdrop");

        assert!(closed.contains(&format!("opacity {SLIDE}")));
        assert!(
            closed.contains(&format!("visibility 0s linear {SLIDE}")),
            "an invisible backdrop that stays visible to hit-testing swallows every tap on the page",
        );
    }

    #[test]
    fn a_long_mostly_horizontal_drag_to_the_right_closes_the_drawer() {
        assert!(closes(140, 6));
        assert!(!closes(-140, 6), "a drag to the left is not a close");
        assert!(!closes(20, 0), "too short to be a swipe");
        assert!(
            !closes(140, 220),
            "mostly vertical is a scroll through the content"
        );
    }

    #[test]
    fn the_drawer_follows_only_a_rightward_mostly_horizontal_drag() {
        assert_eq!(drag_offset(80.0, 5.0), 80.0);
        assert_eq!(
            drag_offset(-80.0, 5.0),
            0.0,
            "the drawer never slides past its open edge"
        );
        assert_eq!(drag_offset(30.0, 120.0), 0.0, "mostly vertical is a scroll");
    }

    #[test]
    fn the_drawer_tracks_the_finger_without_easing_while_dragged() {
        assert!(
            rule(".drawer[data-dragging]").contains("transition: none"),
            "an eased transform restarts on every pointermove and trails the finger",
        );
    }

    #[test]
    fn the_backdrop_clears_as_the_drawer_is_dragged_out() {
        assert_eq!(backdrop_opacity(0.0, 400.0), 1.0);
        assert_eq!(backdrop_opacity(200.0, 400.0), 0.5);
        assert_eq!(backdrop_opacity(600.0, 400.0), 0.0, "past the edge stays clear");
        assert_eq!(
            backdrop_opacity(50.0, 0.0),
            1.0,
            "an unmeasured drawer does not flash clear"
        );
    }

    #[test]
    fn the_backdrop_tracks_the_finger_without_easing_while_dragged() {
        assert!(
            rule(".drawer__backdrop[data-dragging]").contains("transition: none"),
            "an eased opacity restarts on every pointermove and trails the drawer",
        );
    }

    #[test]
    fn only_a_finger_or_a_pen_drags_the_drawer() {
        assert!(drags("touch"));
        assert!(drags("pen"));
        assert!(
            !drags("mouse"),
            "selecting a message with the mouse would otherwise slide the drawer",
        );
    }

    #[test]
    fn the_drawer_leaves_horizontal_drags_to_the_swipe() {
        assert!(
            rule(".drawer").contains("touch-action: pan-y"),
            "without it WebKit pans horizontally itself and cancels the pointer before pointerup",
        );
    }
}
