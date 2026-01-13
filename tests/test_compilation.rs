use gpui::TestAppContext;

#[gpui::test]
fn test_dispatch_exists(cx: &mut TestAppContext) {
    let window = cx.add_window(|_, _| gpui::Empty);
    cx.dispatch_keystroke(window.into(), gpui::Keystroke::parse("a").unwrap());
}