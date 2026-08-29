// Copyright 2026 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;
use std::rc::Rc;

use masonry::core::{
    AccessCtx, ActionCtx, ChildrenIds, ErasedAction, LayoutCtx, MeasureCtx, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{AsUnit, LayoutSize, LenReq, Length, SizeDef};
use masonry::peniko::Color;
use masonry::properties::{Background, TrackColor, types::CrossAxisAlignment};
use masonry::widgets::{Flex, SizedBox, Slider, SliderMoved};

use crate::demo::CONTENT_GAP;

#[derive(PartialEq, Debug, Copy, Clone)]
enum Component {
    R,
    G,
    B,
    A,
}

impl Component {
    const ALL: [Self; 4] = [Self::R, Self::G, Self::B, Self::A];

    fn update(self, color: &mut Color, value: f32) {
        color.components[match self {
            Self::R => 0,
            Self::G => 1,
            Self::B => 2,
            Self::A => 3,
        }] = value;
    }

    fn get(self, color: &Color) -> f32 {
        color.components[match self {
            Self::R => 0,
            Self::G => 1,
            Self::B => 2,
            Self::A => 3,
        }]
    }

    fn visual_color(self) -> Color {
        match self {
            Self::R => Color::from_rgb8(0xff, 0x00, 0x00),
            Self::G => Color::from_rgb8(0x00, 0xff, 0x00),
            Self::B => Color::from_rgb8(0x00, 0x00, 0xff),
            Self::A => Color::WHITE,
        }
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct ColorSelected {
    pub color: Color,
}

pub(crate) struct HorizontalColorPicker {
    color: Color,
    widget: WidgetPod<Flex>,
    sliders: [(Component, Rc<Cell<Option<WidgetId>>>); 4],
    preview_id: Rc<Cell<Option<WidgetId>>>,
}

impl HorizontalColorPicker {
    pub(crate) fn new(color: Color) -> Self {
        let mut body = Flex::row().cross_axis_alignment(CrossAxisAlignment::Stretch);

        let sliders = Component::ALL.map(|component| {
            let id = Cell::new(None).into();
            let id_target = Rc::clone(&id);
            let widget = Slider::new(0., 1., component.get(&color) as f64)
                .prepare()
                .with_props(TrackColor {
                    active: component.visual_color(),
                    ..Default::default()
                })
                .with_id_callback(move |id| id_target.set(Some(id)));
            body = std::mem::replace(&mut body, Flex::row())
                .with(widget, 1.)
                .with_fixed_spacer((CONTENT_GAP.get() / 2.0).px());
            (component, id)
        });

        let preview_id = Cell::new(None).into();
        let preview_id_target = Rc::clone(&preview_id);
        let preview = SizedBox::empty()
            .width(Length::const_px(20.0))
            .prepare()
            .with_props(Background::Color(color))
            .with_id_callback(move |id| preview_id_target.set(Some(id)));
        body = body.with_fixed(preview);

        Self {
            color,
            widget: body.prepare().to_pod(),
            sliders,
            preview_id,
        }
    }

    #[expect(dead_code, reason = "Currently unused, available for future use.")]
    pub(crate) fn set_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        for (component, wid) in this.widget.sliders.iter() {
            let value = component.get(&color);
            this.ctx
                .mutate_later(wid.get().unwrap(), move |mut widget| {
                    if let Some(mut slider_widget) = widget.try_downcast::<Slider>() {
                        Slider::set_value(&mut slider_widget, value as f64);
                    }
                });
        }
        this.ctx
            .mutate_later(this.widget.preview_id.get().unwrap(), move |mut widget| {
                widget.insert_prop(Background::Color(color));
            });
    }
}

impl Widget for HorizontalColorPicker {
    type Action = ColorSelected;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.widget);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let auto_length = len_req.into();
        let context_size = LayoutSize::maybe(axis.cross(), cross_length);
        ctx.compute_length(
            &mut self.widget,
            auto_length,
            context_size,
            axis,
            cross_length,
        )
    }

    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        source: WidgetId,
    ) {
        if let Some(SliderMoved { value }) = action.downcast_ref::<SliderMoved>()
            && let Some(component) = self
                .sliders
                .iter()
                .find(|(_, wid)| Some(source) == wid.get())
                .map(|(comp, _)| *comp)
        {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Value is in range of [0.0, 1.0]"
            )]
            component.update(&mut self.color, *value as f32);
            ctx.submit_action::<Self::Action>(ColorSelected { color: self.color });
            let color = self.color;
            ctx.mutate_later(self.preview_id.get().unwrap(), move |mut widget| {
                widget.insert_prop(Background::Color(color));
            });
            ctx.set_handled();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let content_size = ctx.compute_size(&mut self.widget, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.widget, content_size);
        ctx.place_child(&mut self.widget, Point::ORIGIN);
        ctx.derive_baselines(&self.widget);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
    }

    fn accessibility_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.widget.id()])
    }
}
