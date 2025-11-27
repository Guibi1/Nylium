use std::marker::PhantomData;
use std::rc::Rc;

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::label::Label;
use gpui_component::scroll::Scrollbar;
use gpui_component::scroll::ScrollbarState;
use gpui_component::{ActiveTheme, VirtualListScrollHandle, v_virtual_list};
use nylium_adapter::NyliumServer;
use nylium_adapter::config::FieldOptions;
use nylium_assets::Assets;

use crate::ui::form::{BooleanField, NumberField, StringField};

pub struct SettingsPage<S, C, G>
where
    C: Copy,
    G: Copy,
    S: NyliumServer<C, G>,
{
    fields: Box<[AnyView]>,
    field_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    scroll_state: ScrollbarState,
    _phantoms: PhantomData<S>,
    _phantomc: PhantomData<C>,
    _phantomg: PhantomData<G>,
}

impl<S, C, G> SettingsPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (fields, field_sizes) = generate_fields(window, cx);

        Self {
            fields,
            field_sizes: Rc::new(field_sizes),
            scroll_handle: VirtualListScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
            _phantoms: PhantomData,
            _phantomc: PhantomData,
            _phantomg: PhantomData,
        }
    }
}

impl<S, C, G> Render for SettingsPage<S, C, G>
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_grow()
            .px_4()
            .pt_4()
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(Label::new("Server settings").text_xl())
                            .child(
                                Label::new("A restart is required for changes to take effect.")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(Button::new("reload").icon(Assets::Rotate).ghost().on_click(
                        cx.listener(|this, _, window, cx| {
                            let (fields, field_sizes) = generate_fields(window, cx);
                            this.fields = fields;
                            this.field_sizes = Rc::new(field_sizes);
                            cx.notify();
                        }),
                    )),
            )
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "game_rules_list",
                    self.field_sizes.clone(),
                    |this, range, _, _| {
                        range
                            .filter_map::<AnyElement, _>(|i| {
                                this.fields
                                    .get(i)
                                    .map(|field| field.clone().into_any_element())
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.scroll_handle)
                .px_4(),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(Scrollbar::vertical(&self.scroll_state, &self.scroll_handle)),
            )
    }
}

fn generate_fields<S, C, G>(
    window: &mut Window,
    cx: &mut Context<SettingsPage<S, C, G>>,
) -> (Box<[AnyView]>, Vec<Size<Pixels>>)
where
    C: Copy + 'static,
    G: Copy + 'static,
    S: NyliumServer<C, G>,
{
    let mut field_sizes = Vec::new();
    let fields: Box<[AnyView]> = cx
        .global::<S>()
        .get_config()
        .into_iter()
        .map(|option| {
            let value = cx.global::<S>().get_config_value(option.key());
            match option {
                FieldOptions::Boolean(option) => {
                    field_sizes.push(BooleanField::get_height());
                    let field =
                        cx.new(|_| BooleanField::new(option.label, value.assert_bool(), option.id));
                    cx.subscribe(&field, move |_, _, event, cx| {
                        cx.global_mut::<S>()
                            .set_config_value(option.key, event.value.clone())
                    })
                    .detach();
                    field.into()
                }
                FieldOptions::Number(option) => {
                    field_sizes.push(NumberField::get_height());
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
                            .set_config_value(option.key, event.value.clone())
                    })
                    .detach();
                    field.into()
                }
                FieldOptions::String(option) => {
                    field_sizes.push(StringField::get_height());
                    let field = cx.new(|cx| {
                        StringField::new(option.label, value.assert_string(), window, cx)
                    });
                    cx.subscribe(&field, move |_, _, event, cx| {
                        cx.global_mut::<S>()
                            .set_config_value(option.key, event.value.clone())
                    })
                    .detach();
                    field.into()
                }
            }
        })
        .collect();

    field_sizes.push(Size::new(px(0.), px(8.)));
    (fields, field_sizes)
}
