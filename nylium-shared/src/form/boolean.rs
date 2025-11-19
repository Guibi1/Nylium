use std::rc::Rc;

use gpui::*;
use gpui_component::switch::Switch;

pub struct BooleanField<C: Global> {
    key: SharedString,
    get: Rc<dyn Fn(&C) -> bool>,
    set: Rc<dyn Fn(bool, &mut C)>,
}

impl<C> BooleanField<C>
where
    C: Global,
{
    pub fn new<G, S>(key: SharedString, get: G, set: S) -> Self
    where
        G: Fn(&C) -> bool + 'static,
        S: Fn(bool, &mut C) + 'static,
    {
        BooleanField {
            key,
            get: Rc::new(get),
            set: Rc::new(set),
        }
    }
}

impl<C> Render for BooleanField<C>
where
    C: Global,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = (self.get)(cx.global::<C>());

        Switch::new(self.key.clone())
            .checked(checked)
            .on_click(cx.listener(move |this, checked, _window, cx| {
                (this.set)(*checked, cx.global_mut::<C>());
            }))
    }
}
