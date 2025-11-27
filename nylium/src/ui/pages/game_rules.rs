use gpui::*;
use gpui_component::StyledExt;
use gpui_component::label::Label;
use nylium_adapter::NyliumServer;
use nylium_adapter::config::FieldOptions;

use crate::ui::form::{BooleanField, NumberField, StringField};

pub struct GameRulesPage {
    fields: Box<[AnyView]>,
}

impl GameRulesPage {
    pub fn new<S, C, G>(window: &mut Window, cx: &mut Context<Self>) -> Self
    where
        C: Copy + 'static,
        G: Copy + 'static,
        S: NyliumServer<C, G>,
    {
        Self {
            fields: cx
                .global::<S>()
                .get_gamerules()
                .into_iter()
                .map(|option| {
                    let value = cx.global::<S>().get_gamerule_value(option.key());
                    match option {
                        FieldOptions::Boolean(option) => {
                            let field = cx.new(|_| {
                                BooleanField::new(option.label, value.assert_bool(), option.id)
                            });
                            cx.subscribe(&field, move |_, _, event, cx| {
                                cx.global_mut::<S>()
                                    .set_gamerule_value(option.key, event.value.clone())
                            })
                            .detach();
                            field.into()
                        }
                        FieldOptions::Number(option) => {
                            let field = cx.new(|cx| {
                                NumberField::new(
                                    option.label,
                                    value.assert_number(),
                                    option.min,
                                    option.max,
                                    window,
                                    cx,
                                )
                            });
                            cx.subscribe(&field, move |_, _, event, cx| {
                                cx.global_mut::<S>()
                                    .set_gamerule_value(option.key, event.value.clone())
                            })
                            .detach();
                            field.into()
                        }
                        FieldOptions::String(option) => {
                            let field = cx.new(|cx| {
                                StringField::new(option.label, value.assert_string(), window, cx)
                            });
                            cx.subscribe(&field, move |_, _, event, cx| {
                                cx.global_mut::<S>()
                                    .set_gamerule_value(option.key, event.value.clone())
                            })
                            .detach();
                            field.into()
                        }
                    }
                })
                .collect(),
        }
    }
}

impl Render for GameRulesPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_2().overflow_hidden().child(
            div()
                .p_4()
                .scrollable(Axis::Vertical)
                .child(Label::new("Game rules").text_xl())
                .child(
                    div()
                        .p_4()
                        .pt_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(self.fields.iter().cloned()),
                ),
        )
    }
}
