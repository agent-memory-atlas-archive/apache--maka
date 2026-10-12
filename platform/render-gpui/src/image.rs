/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

//! Original native image custody; immutable recipes never read URLs or paths.
mod decode;
use crate::{Input, LiveInput, NativeBudget, Slots, Widget, appearance::color};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use decode::{Decoded, Encoded};
use gpui_kit::accesskit::Role;
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, ImageSource as PaintSource, InteractiveElement,
    IntoElement, ObjectFit, ParentElement, Render, RenderImage, Styled, Window, div, img, px,
};
use gpui_kit::{StatefulInteractiveElement, StyledImage};
use maka_ui_runtime::{
    ComponentCall, ComponentOutput, Emission, Frame, NativeCapability, NativeComponent,
    NativeExport, NativeRegistry, NativeView,
    budget::{Kind, Permit},
    image::*,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{any::Any, collections::BTreeMap, rc::Rc, sync::Arc};

struct Pixels {
    image: Arc<RenderImage>,
    _permit: Permit,
    _keep: Input,
}
struct Owner {
    id: String,
    input: LiveInput,
    props: ImageProps,
    buffer: Option<Encoded>,
    pixels: Option<Rc<Pixels>>,
    observation: ImageObservation,
    #[cfg(test)]
    decode_gate: Option<tokio::sync::oneshot::Receiver<()>>,
    source_revision: u64,
    last_packet: Option<(u64, u64, [u8; 32])>,
    pending: bool,
    needs_publication: bool,
}
impl Owner {
    fn new(props: &ImageProps, input: LiveInput, cx: &mut App) -> Entity<Self> {
        let keep = input.0.borrow().as_ref().expect("live image mount").clone();
        let id = uuid::Uuid::new_v4().to_string();
        cx.new(|cx| {
            cx.on_release(move |owner: &mut Owner, cx| {
                if let Some(pixels) = owner.pixels.take() {
                    cx.drop_image(pixels.image.clone(), None);
                }
                drop(keep);
            })
            .detach();
            let mut initial = props.clone();
            initial.source = None;
            initial.packet = None;
            Self {
                id: id.clone(),
                input,
                props: initial,
                buffer: None,
                pixels: None,
                observation: ImageObservation {
                    owner: id,
                    revision: 0,
                    source: None,
                    received: 0,
                    phase: ImagePhase::Empty,
                    width: None,
                    height: None,
                    error: None,
                    receipt: None,
                },
                #[cfg(test)]
                decode_gate: None,
                source_revision: 0,
                last_packet: None,
                pending: false,
                needs_publication: true,
            }
        })
    }
    fn changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.observation.revision = self
            .observation
            .revision
            .checked_add(1)
            .expect("image revision");
        self.needs_publication = true;
        self.schedule(window, cx);
        cx.notify();
    }
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending || !self.needs_publication || self.input.0.borrow().is_none() {
            return;
        }
        self.pending = true;
        let weak = cx.weak_entity();
        window.defer(cx, move |window, cx| {
            let Ok(Some((input, value))) = weak.update(cx, |owner, _| {
                owner.pending = false;
                if !owner.needs_publication {
                    return None;
                }
                Some((owner.input.0.borrow().clone()?, owner.observation.clone()))
            }) else {
                return;
            };
            if (input.0)(
                "observe",
                serde_json::to_value(&value).expect("image observation"),
                window,
                cx,
            )
            .is_ok()
            {
                let _ = weak.update(cx, |owner, _| {
                    if owner.observation == value {
                        owner.needs_publication = false;
                    }
                });
            }
        });
    }
    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.buffer = None;
        if let Some(pixels) = self.pixels.take() {
            cx.drop_image(pixels.image.clone(), Some(window));
        }
        self.observation.received = 0;
        self.observation.width = None;
        self.observation.height = None;
        self.observation.error = None;
        self.observation.receipt = None;
    }
    fn apply(&mut self, props: &ImageProps, window: &mut Window, cx: &mut Context<Self>) {
        if self.input.0.borrow().is_none() {
            return;
        }
        if self.props.source != props.source {
            self.source_revision = self
                .source_revision
                .checked_add(1)
                .expect("image source revision");
            let conflict = self
                .props
                .source
                .as_ref()
                .zip(props.source.as_ref())
                .is_some_and(|(old, new)| old.key == new.key);
            self.clear(window, cx);
            self.observation.source = props.source.as_ref().map(|s| s.key.clone());
            self.observation.phase = ImagePhase::Empty;
            if let Some(source) = &props.source {
                let result = if conflict {
                    Err(ImageError::SourceConflict)
                } else {
                    usize::try_from(source.total_bytes)
                        .ok()
                        .filter(|n| *n > 0 && *n <= decode::MAX_BYTES)
                        .ok_or(ImageError::LimitExceeded)
                        .and_then(|bytes| {
                            let permit = self.input.1.reserve(Kind::Encoded, bytes)?;
                            Ok(Encoded {
                                bytes: Vec::with_capacity(bytes),
                                _permit: permit,
                            })
                        })
                };
                match result {
                    Ok(buffer) => {
                        self.buffer = Some(buffer);
                        self.observation.phase = ImagePhase::Receiving;
                    }
                    Err(error) => self.fail(error),
                }
            }
            self.changed(window, cx);
        }
        let mut next = props.clone();
        next.packet = None;
        if self.props != next {
            self.props = next;
            cx.notify();
        }
        if let Some(packet) = &props.packet {
            self.packet(packet, window, cx);
        }
        self.schedule(window, cx);
    }
    fn fail(&mut self, error: ImageError) {
        self.buffer = None;
        self.observation.phase = ImagePhase::Failed;
        self.observation.error = Some(error);
    }
    fn packet(&mut self, packet: &ImagePacket, window: &mut Window, cx: &mut Context<Self>) {
        let signature: [u8; 32] =
            Sha256::digest(serde_json::to_vec(packet).expect("image packet")).into();
        if self.last_packet == Some((packet.sequence, self.source_revision, signature)) {
            self.schedule(window, cx);
            return;
        }
        let result = (|| {
            if packet.owner != self.id
                || self
                    .props
                    .source
                    .as_ref()
                    .is_none_or(|source| source.key != packet.source)
                || self
                    .last_packet
                    .is_some_and(|(sequence, _, _)| packet.sequence <= sequence)
                || packet.offset != self.observation.received
                || self.observation.phase != ImagePhase::Receiving
                || packet.data.len() > 43692
            {
                return Err(ImageError::PacketRejected);
            }
            let bytes = STANDARD
                .decode(&packet.data)
                .map_err(|_| ImageError::PacketRejected)?;
            let source = self.props.source.as_ref().expect("checked source");
            if bytes.is_empty()
                || bytes.len() > 32 * 1024
                || (bytes.len() as u64) > source.total_bytes - packet.offset
            {
                return Err(ImageError::PacketRejected);
            }
            self.buffer
                .as_mut()
                .ok_or(ImageError::PacketRejected)?
                .bytes
                .extend_from_slice(&bytes);
            self.observation.received += bytes.len() as u64;
            Ok(())
        })();
        // Keep only request identity, never a second encoded packet buffer.
        if packet.owner == self.id
            && self
                .props
                .source
                .as_ref()
                .is_some_and(|s| s.key == packet.source)
            && self
                .last_packet
                .is_none_or(|(sequence, _, _)| packet.sequence > sequence)
        {
            self.last_packet = Some((packet.sequence, self.source_revision, signature));
        }
        self.observation.receipt = Some(ImageReceipt {
            owner: packet.owner.clone(),
            source: packet.source.clone(),
            sequence: packet.sequence,
            accepted: result.is_ok(),
            error: result.err(),
        });
        if result.is_ok()
            && self
                .props
                .source
                .as_ref()
                .is_some_and(|s| self.observation.received == s.total_bytes)
        {
            self.start_decode(window, cx);
        }
        self.changed(window, cx);
    }
    fn start_decode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let budget = self.input.1.clone();
        let job = match budget.reserve(Kind::Job, 1) {
            Ok(job) => job,
            Err(error) => {
                self.fail(error.into());
                return;
            }
        };
        let encoded = self.buffer.take().expect("received image");
        let source = self.props.source.clone().expect("received source");
        let expected = source.clone();
        let source_revision = self.source_revision;
        #[cfg(test)]
        let gate = self.decode_gate.take();
        let task = cx.background_executor().spawn(async move {
            #[cfg(test)]
            if let Some(gate) = gate {
                let _ = gate.await;
            }
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decode::decode(encoded, source, budget, job)
            }))
            .unwrap_or(Err(ImageError::DecodeFailed))
        });
        self.observation.phase = ImagePhase::Decoding;
        let weak = cx.weak_entity();
        // Detach the original admitted task, not its lease. It drains even if the
        // widget disappears or source changes while the worker is decoding.
        let keep = self
            .input
            .0
            .borrow()
            .as_ref()
            .expect("admitted image owner")
            .clone();
        window
            .spawn(cx, async move |cx| {
                let result = task.await;
                let _ = cx.update(move |window, cx| {
                    let _ = weak.update(cx, |owner, cx| {
                        if owner.input.0.borrow().is_none()
                            || owner.source_revision != source_revision
                            || owner.props.source.as_ref() != Some(&expected)
                        {
                            return;
                        }
                        match result {
                            Ok(Decoded { image, permit }) => {
                                let size = image.size(0);
                                owner.observation.width =
                                    Some(u32::try_from(size.width.0).expect("bounded image width"));
                                owner.observation.height = Some(
                                    u32::try_from(size.height.0).expect("bounded image height"),
                                );
                                owner.pixels = Some(Rc::new(Pixels {
                                    image,
                                    _permit: permit,
                                    _keep: keep.clone(),
                                }));
                                owner.observation.phase = ImagePhase::Ready;
                            }
                            Err(error) => owner.fail(error),
                        }
                        owner.changed(window, cx);
                    });
                    drop(keep);
                });
            })
            .detach();
    }
}
impl Render for Owner {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let props = &self.props;
        let mut element = div()
            .id("owned-image")
            .w_full()
            .h(px(props.height))
            .overflow_hidden()
            .rounded(px(props.radius))
            .bg(color(props.background))
            .border_1()
            .border_color(color(props.border))
            .role(Role::Image)
            .aria_label(props.alt.clone());
        if let Some(pixels) = &self.pixels {
            let pixels = pixels.clone();
            let fit = match props.fit {
                ImageFit::Contain => ObjectFit::Contain,
                ImageFit::Cover => ObjectFit::Cover,
            };
            // GPUI requires Arc for this foreground-only callback; its original
            // Input custody is Rc and must remain on the presentation thread.
            #[allow(clippy::arc_with_non_send_sync)]
            let source = PaintSource::Custom(Arc::new(move |_, _| Some(Ok(pixels.image.clone()))));
            element = element.child(img(source).size_full().object_fit(fit));
        }
        element
    }
}
struct Recipe(ImageProps);
impl Widget for Recipe {
    fn create(&self, input: LiveInput, _: &mut Window, cx: &mut App) -> Box<dyn Any> {
        Box::new(Owner::new(&self.0, input, cx))
    }
    fn commit(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("image owner")
            .update(cx, |owner, cx| owner.apply(&self.0, window, cx));
    }
    fn publish_pending(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("image owner")
            .update(cx, |owner, cx| owner.schedule(window, cx));
    }
    fn deactivate(&self, state: &mut dyn Any, window: &mut Window, cx: &mut App) {
        state
            .downcast_ref::<Entity<Owner>>()
            .expect("image owner")
            .update(cx, |owner, cx| owner.clear(window, cx));
    }
    fn render(
        &self,
        state: &dyn Any,
        _: &Slots,
        _: &mut Window,
        _: &mut App,
    ) -> maka_ui_runtime::Result<AnyElement> {
        Ok(state
            .downcast_ref::<Entity<Owner>>()
            .expect("image owner")
            .clone()
            .into_any_element())
    }
}
struct ImageComponent;
impl NativeComponent<dyn Widget> for ImageComponent {
    fn call(
        &self,
        call: ComponentCall<'_>,
        _: &maka_ui_runtime::abi::ValueBindings,
    ) -> maka_ui_runtime::Result<ComponentOutput<NativeView<dyn Widget>>> {
        let (props, state, event) = match call {
            ComponentCall::Mount { props } => (props, None, None),
            ComponentCall::Update { state, props } => (props, Some(state), None),
            ComponentCall::Event {
                state,
                props,
                input,
                ..
            } => (props, Some(state), Some(input)),
        };
        let mut props: ImageProps = serde_json::from_value(props.clone())
            .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
        let revision = state.map_or(0, |s| {
            s["revision"].as_u64().expect("image frame revision") + 1
        });
        let mut request: Option<ImagePacket> = state
            .and_then(|s| serde_json::from_value(s["request"].clone()).ok())
            .flatten();
        let mut pending = state.is_some_and(|s| s["pending"].as_bool().unwrap_or(false));
        let observation = event
            .cloned()
            .unwrap_or_else(|| state.map_or(Value::Null, |s| s["observation"].clone()));
        if let Some(packet) = &props.packet
            && request.as_ref() != Some(packet)
        {
            if request
                .as_ref()
                .is_some_and(|r| packet.sequence <= r.sequence)
            {
                return Err(maka_ui_runtime::Error::Invalid(
                    "image packet sequence must advance".into(),
                ));
            }
            request = Some(packet.clone());
            pending = true;
        }
        if let Some(event) = event {
            let value: ImageObservation = serde_json::from_value(event.clone())
                .map_err(|e| maka_ui_runtime::Error::Invalid(e.to_string()))?;
            if request
                .as_ref()
                .zip(value.receipt.as_ref())
                .is_some_and(|(r, receipt)| {
                    r.owner == receipt.owner
                        && r.source == receipt.source
                        && r.sequence == receipt.sequence
                })
            {
                pending = false;
            }
        }
        props.packet = request
            .clone()
            .filter(|r| pending && props.source.as_ref().is_some_and(|s| s.key == r.source));
        let events = event.map_or_else(Vec::new, |e| {
            vec![Emission {
                name: "observed".into(),
                value: e.clone(),
            }]
        });
        Ok(Frame {revision,state:json!({"revision":revision,"request":request,"pending":pending,"observation":observation}),surface:None,
            view:NativeView::<dyn Widget>::new(Rc::new(Recipe(props)),BTreeMap::from([("observe".into(),observation_schema())]),vec![])?,
        }.output(events))
    }
}
pub(super) fn register(registry: &mut NativeRegistry<dyn Widget>) {
    registry
        .register(maka_ui_runtime::image::IMPLEMENTATION, |_, _| {
            Ok(BTreeMap::from([(
                "Image".into(),
                NativeExport {
                    contract: image_contract(),
                    value: NativeCapability::Component(Rc::new(ImageComponent)),
                },
            )]))
        })
        .expect("unique native image implementation");
}
#[cfg(test)]
mod tests;
