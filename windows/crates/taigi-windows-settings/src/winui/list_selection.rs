// A ListView selection index handed to XAML only once the rows it counts are there.

use windows_reactor::{
    Component, KeyedView, ListView, ListViewSlot, SlotsControl, View, ViewContext,
};

/// The rows XAML is known to hold, so a selection index is never applied to a
/// list that is still one render behind it.
///
/// Reactor plans a node's PROPERTIES before its children: for a `ListView`
/// whose items live in a collection slot, `SetProperty(ListViewSelectedIndex)`
/// is emitted ahead of the item inserts of the same render
/// (`planner/view.rs` `ViewKind::Slots` reconciles the shallow control first,
/// then the keyed child list). An index the native list does not number yet is
/// `E_INVALIDARG` from XAML's setter, and reactor treats a failed native
/// command as fatal — `pump_error` calls `std::process::abort()`, which is a
/// silent `0xc0000409` with no panic to read. That is what killed the settings
/// window the first time a user added a custom typeface: five bundled rows on
/// screen, index 5 selected in the render that appended the sixth.
///
/// So the selection is handed over in two turns. The render that changes the
/// rows draws them with NO selection (XAML's `-1`); an effect — which reactor
/// runs only after the native commands of that render applied
/// (`pump/publish.rs`: `apply_native_commands` then `commit_component_effects`)
/// — reports the rows back, and the render after that carries the index. The
/// stored preference is never touched: this is about what XAML is told, not
/// about what the user chose.
///
/// What is held is the ROWS OF ONE NATIVE LIST, not the page's data: the
/// effect's cleanup clears it, so a list that goes away with its pane starts
/// its next mount unsettled. Nothing else may write it — a model that
/// remembered rows across a remount would hand the fresh list an index again
/// in its very first batch.
#[derive(Default)]
pub struct SettledRows {
    /// The keys of the rows the live list holds, or `None` while no report
    /// stands for it — before its first, and after it was taken down.
    applied: Option<Vec<String>>,
}

impl SettledRows {
    /// The index to render for the rows `keys` identifies: the caller's
    /// `desired` once XAML holds exactly those rows, and `None` until then.
    fn selection(&self, keys: &[String], desired: Option<usize>) -> Option<usize> {
        if self.applied.as_deref() == Some(keys) {
            desired
        } else {
            None
        }
    }

    /// The key of the row at `index` OF THE LIST ON SCREEN — what a selection
    /// event names.
    ///
    /// A selection event carries an index into the rows XAML holds, which are
    /// not always the rows the model holds: a background reload can land
    /// between the click and its message. Resolving through here turns the
    /// index back into the row the user pressed; a page then looks that row up
    /// in its own data and drops the event if it is gone. `None` while the
    /// list is mid-handover, which is where our own `-1` comes back.
    pub fn key_at(&self, index: usize) -> Option<&str> {
        self.applied.as_ref()?.get(index).map(String::as_str)
    }

    /// What a render's effect reports: the rows it applied, or `None` from the
    /// cleanup that runs when those rows are replaced or the list is taken
    /// down. A report for rows that have since changed again settles nothing —
    /// the newer render declares its own effect.
    pub fn report(&mut self, rows: Option<Vec<String>>) {
        self.applied = rows;
    }
}

/// A `ListView` whose selection is handed over safely: `items` go in, the
/// index follows a render later.
///
/// The keys come OUT of `items`, so the rows reported back are the rows XAML
/// was given — a list cannot be half-guarded, and there is no second key list
/// beside the items to drift from them. `list` is the caller's own
/// `ListView`, configured with everything but its items and its selection;
/// `desired` is the index the page would like; `report` wraps a report for
/// this list's [`SettledRows::report`], under an `effect_key` unique in the
/// component.
pub fn selectable_list<C: Component, Item>(
    effect_key: &'static str,
    settled: &SettledRows,
    items: Vec<(String, Item)>,
    desired: Option<usize>,
    list: ListView,
    context: &mut ViewContext<C>,
    report: impl Fn(Option<Vec<String>>) -> C::Message + 'static,
) -> View
where
    (String, Item): Into<KeyedView>,
{
    let keys: Vec<String> = items.iter().map(|(key, _)| key.clone()).collect();
    track_rows(effect_key, &keys, context, report);
    list.selected_index(settled.selection(&keys, desired))
        .collection_slot(ListViewSlot::Items, items)
}

/// Declares the effect that reports `keys` back once XAML has them, and
/// withdraws the report when they are replaced or the list goes away.
fn track_rows<C: Component>(
    key: &'static str,
    keys: &[String],
    context: &mut ViewContext<C>,
    message: impl Fn(Option<Vec<String>>) -> C::Message + 'static,
) {
    let applied = keys.to_vec();
    let sender = context.sender();
    context.use_effect(key, applied.clone(), move || {
        sender.send(message(Some(applied)));
        Some(Box::new(move || {
            sender.send(message(None));
        }))
    });
}

/// Reading a recorded plan: what the runtime was told about a list's items and
/// its selection. Shared by this module's own reactor tests and by
/// `winui::pane_planning`, so one spelling of the two `Command` shapes serves
/// both nets.
#[cfg(test)]
pub mod recorded {
    use windows_reactor::{Command, NodeId, PropertyId, PropertyValue};

    /// The nodes these commands gave children to.
    pub fn insertion_parents(commands: &[Command]) -> Vec<NodeId> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::InsertChild { parent, .. } => Some(*parent),
                _ => None,
            })
            .collect()
    }

    /// How many items these commands inserted.
    pub fn inserted(commands: &[Command]) -> usize {
        insertion_parents(commands).len()
    }

    /// Every list selection these commands set, as (list, index) pairs.
    pub fn selections(commands: &[Command]) -> Vec<(NodeId, usize)> {
        commands
            .iter()
            .filter_map(|command| match command {
                Command::SetProperty {
                    node,
                    property: PropertyId::ListViewSelectedIndex,
                    value: PropertyValue::SelectionIndex(Some(index)),
                } => Some((*node, *index)),
                _ => None,
            })
            .collect()
    }

    /// The last thing these commands said about a list's selection: `None` if
    /// they said nothing, `Some(None)` for XAML's "no selection".
    pub fn last_selection(commands: &[Command]) -> Option<Option<usize>> {
        commands.iter().rev().find_map(|command| match command {
            Command::SetProperty {
                property: PropertyId::ListViewSelectedIndex,
                value: PropertyValue::SelectionIndex(index),
                ..
            } => Some(*index),
            Command::ClearProperty {
                property: PropertyId::ListViewSelectedIndex,
                ..
            } => Some(None),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// The render that grows the list draws no selection, whatever it wants:
    /// this is the crash — five rows on screen, index 5 asked for.
    #[test]
    fn a_row_added_this_render_is_not_selected_yet() {
        let mut settled = SettledRows::default();
        settled.report(Some(keys(&["a", "b"])));

        let grown = keys(&["a", "b", "c"]);

        assert_eq!(settled.selection(&grown, Some(2)), None);
    }

    /// The turn after the effect reports them, the index goes over.
    #[test]
    fn the_index_lands_once_the_rows_are_acknowledged() {
        let mut settled = SettledRows::default();
        let rows = keys(&["a", "b", "c"]);

        settled.report(Some(rows.clone()));

        assert_eq!(settled.selection(&rows, Some(2)), Some(2));
    }

    /// Nothing is reported before the first render, so a list that mounts with
    /// a selection still hands XAML its rows first.
    #[test]
    fn a_fresh_list_starts_unsettled() {
        let settled = SettledRows::default();

        assert_eq!(settled.selection(&keys(&["a"]), Some(0)), None);
        assert_eq!(settled.key_at(0), None);
    }

    /// The list came down with its pane: the next mount is a new native list,
    /// which must be handed its rows before any index.
    #[test]
    fn a_withdrawn_report_leaves_the_next_mount_unsettled() {
        let mut settled = SettledRows::default();
        let rows = keys(&["a", "b"]);
        settled.report(Some(rows.clone()));

        settled.report(None);

        assert_eq!(settled.selection(&rows, Some(1)), None);
    }

    /// Rows that changed without growing are as unsafe as a longer list — the
    /// index would name a different row — and so is a report that arrives
    /// after the rows moved on again.
    #[test]
    fn rows_that_do_not_match_the_report_are_unsettled() {
        let mut settled = SettledRows::default();
        settled.report(Some(keys(&["a", "b"])));

        assert_eq!(settled.selection(&keys(&["b", "a"]), Some(1)), None);
        assert_eq!(settled.selection(&keys(&["a", "b", "c"]), Some(2)), None);
    }

    /// An empty list is a settled state of its own — the rows really are none.
    #[test]
    fn an_emptied_list_settles_on_no_rows() {
        let mut settled = SettledRows::default();
        settled.report(Some(keys(&["a"])));

        settled.report(Some(Vec::new()));

        assert_eq!(settled.selection(&[], None), None);
        assert_eq!(settled.key_at(0), None);
    }

    /// A selection event names the row the user pressed on the list XAML
    /// holds, not the row that index has in the model's newer data.
    #[test]
    fn a_selection_event_resolves_against_the_rows_on_screen() {
        let mut settled = SettledRows::default();
        settled.report(Some(keys(&["a", "b", "c"])));

        assert_eq!(settled.key_at(2), Some("c"));
        assert_eq!(settled.key_at(3), None);
    }
}

/// The handover, driven through the reactor's headless host: what the native
/// list is actually told, in order, across the renders a growing list takes.
///
/// The unit tests above pin the rule; this pins that reactor plays it out —
/// the property really is planned before the items, one effect turn really
/// does settle the rows, and a list that is taken down and put back really
/// does start over.
#[cfg(test)]
mod reactor_tests {
    use super::recorded::{inserted, last_selection};
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use windows_reactor::*;

    #[derive(Clone)]
    struct Handle(Rc<RefCell<Option<LocalSender<Message>>>>);

    impl PartialEq for Handle {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.0, &other.0)
        }
    }

    #[derive(Clone)]
    enum Message {
        /// One more row, selected — the shape that killed the settings window.
        Grow,
        /// The list leaves the tree and comes back, as a pane switch does.
        Remount(bool),
        RowsApplied(Option<Vec<String>>),
    }

    struct GrowingList {
        rows: usize,
        is_shown: bool,
        settled: SettledRows,
    }

    impl GrowingList {
        fn keys(&self) -> Vec<String> {
            (0..self.rows).map(|row| format!("row.{row}")).collect()
        }
    }

    impl Component for GrowingList {
        type Input = Handle;
        type Message = Message;

        fn create(input: &Self::Input, context: &ComponentContext<Self>) -> Self {
            *input.0.borrow_mut() = Some(context.sender());
            Self {
                rows: 2,
                is_shown: true,
                settled: SettledRows::default(),
            }
        }

        fn update(&mut self, message: Self::Message, _context: &ComponentContext<Self>) {
            match message {
                Message::Grow => self.rows += 1,
                Message::Remount(is_shown) => self.is_shown = is_shown,
                Message::RowsApplied(rows) => self.settled.report(rows),
            }
        }

        fn view(&self, _input: &Self::Input, context: &mut ViewContext<Self>) -> View {
            if !self.is_shown {
                return View::empty();
            }
            let items = self
                .keys()
                .into_iter()
                .map(|key| (key.clone(), ListViewItem::new().tag(key)))
                .collect::<Vec<_>>();
            // Always the last row: every growth asks for an index the native
            // list does not hold yet.
            let desired = items.len().checked_sub(1);
            selectable_list(
                "rows",
                &self.settled,
                items,
                desired,
                ListView::new().selection_mode(ListViewSelectionMode::Single),
                context,
                Message::RowsApplied,
            )
        }
    }

    /// Everything the runtime was told since the last look, oldest first.
    fn since(pump: &Pump<RecordingRuntime>, consumed: &mut usize) -> Vec<Command> {
        let batches = pump.runtime().commands();
        let fresh = batches[*consumed..].concat();
        *consumed = batches.len();
        fresh
    }

    fn mount() -> (Pump<RecordingRuntime>, Handle) {
        let handle = Handle(Rc::new(RefCell::new(None)));
        let mut pump = Pump::new(RecordingRuntime::default());
        pump.mount_view(View::component::<GrowingList>(handle.clone()))
            .expect("the list mounts");
        (pump, handle)
    }

    fn send(handle: &Handle, message: Message) {
        assert!(
            handle.0.borrow().as_ref().expect("a sender").send(message),
            "the component takes the message",
        );
    }

    /// One turn of the component queue, which is where an effect's report is
    /// answered.
    fn turn(pump: &mut Pump<RecordingRuntime>) {
        pump.dispatch_components(8).expect("a component turn");
    }

    #[test]
    fn a_mount_hands_over_its_rows_before_any_index() {
        let (mut pump, handle) = mount();
        let mut seen = 0;

        let mounting = since(&pump, &mut seen);
        assert!(inserted(&mounting) >= 2, "the rows were inserted");
        assert!(
            matches!(last_selection(&mounting), None | Some(None)),
            "a fresh list must be told its rows before an index",
        );

        // The mount's effect reported the rows; the render it wakes carries
        // the selection, with no item command beside it.
        turn(&mut pump);
        let settling = since(&pump, &mut seen);
        assert_eq!(last_selection(&settling), Some(Some(1)));
        assert_eq!(inserted(&settling), 0);

        drop(handle);
    }

    #[test]
    fn a_grown_list_takes_its_index_only_after_the_row_landed() {
        let (mut pump, handle) = mount();
        let mut seen = 0;
        turn(&mut pump);
        since(&pump, &mut seen);

        send(&handle, Message::Grow);
        turn(&mut pump);
        let growing = since(&pump, &mut seen);
        assert_eq!(inserted(&growing), 1, "the third row went in");
        assert!(
            matches!(last_selection(&growing), None | Some(None)),
            "the render that inserts a row must not select it",
        );

        turn(&mut pump);
        let settling = since(&pump, &mut seen);
        assert_eq!(last_selection(&settling), Some(Some(2)));
        assert_eq!(inserted(&settling), 0);
    }

    /// The defect Codex found in the first cut: the model outlived the native
    /// list, so a pane the user came back to was handed an index in its very
    /// first batch.
    #[test]
    fn a_remounted_list_starts_over() {
        let (mut pump, handle) = mount();
        let mut seen = 0;
        turn(&mut pump);
        since(&pump, &mut seen);

        send(&handle, Message::Remount(false));
        turn(&mut pump);
        // The cleanup's withdrawal is a message of its own.
        turn(&mut pump);
        since(&pump, &mut seen);

        send(&handle, Message::Remount(true));
        turn(&mut pump);
        let remounting = since(&pump, &mut seen);
        assert!(inserted(&remounting) >= 2, "the list was built again");
        assert!(
            matches!(last_selection(&remounting), None | Some(None)),
            "a list built again is as new as the first one",
        );

        turn(&mut pump);
        let settling = since(&pump, &mut seen);
        assert_eq!(last_selection(&settling), Some(Some(1)));
    }
}
