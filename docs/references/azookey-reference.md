# azooKey Reference Research

> **Type**: Reference
> **Keywords**: `azooKey`, `SwiftUI`, `Flick`, `CustardKit`
> **Related**: rime-reference.md, khiin-reference.md

---

## Summary

- iOS Japanese keyboard, implemented with SwiftUI
- Dual-track layout system: CustardKit (Flick) + UnifiedKey (QWERTY)
- 47 action types with unified interface
- Main App uses TabView + NavigationStack architecture

---

## Keyboard Architecture

### Dual-track Layout System

| System | For | Coordinates |
|--------|-----|-------------|
| CustardKit | Flick keyboard | Integer gridFit (x, y) |
| UnifiedKey | QWERTY | Float + width |

### CustardKit Example

```swift
Custard(
    identifier: "japanese_flick",
    interface: CustardInterface(
        keyLayout: .gridFit(.init(rowCount: 5, columnCount: 4)),
        keys: [
            .gridFit(.init(x: 1, y: 1)): .custom(
                .flickSimpleInputs(
                    center: "あ", left: "い", top: "う",
                    right: "え", bottom: "お"
                )
            )
        ]
    )
)
```

### Key Model Protocol

```swift
protocol UnifiedKeyModelProtocol {
    func pressActions(variableStates) -> [ActionType]
    func longPressActions(variableStates) -> LongpressActionType
    func variationSpace(variableStates) -> UnifiedVariationSpace
}
```

---

## Action Types

| Category | Action | Use |
|----------|--------|-----|
| Text input | `.input(String)` | Input text |
| Delete | `.delete(Int)` | Delete characters |
| Cursor | `.moveCursor(Int)` | Move cursor |
| Tab | `.moveTab(TabData)` | Switch keyboard |
| State | `.setBoolState(String, BoolOperation)` | Caps Lock |

### Variation Space

```swift
enum UnifiedVariationSpace {
    case none
    case fourWay([FlickDirection: UnifiedVariation])  // Flick four-way
    case linear([VariationElement], direction: ...)   // Long-press popup
}
```

---

## Main App Architecture

### TabView Structure

| Tab | Icon | Function |
|-----|------|----------|
| Tips | `lightbulb.fill` | Usage guide |
| Theme | `photo` | Themes |
| Customize | `gearshape.2.fill` | Customization |
| Settings | `wrench.fill` | Settings |

### NavigationStack Pattern

```swift
enum Path: Hashable {
    case edit(index: Int?)
    case information(String)
}

NavigationStack(path: $path) {
    .navigationDestination(for: Path.self) { destination in
        switch destination { ... }
    }
}
```

### Onboarding Flow

| Step | Description |
|------|-------------|
| menu | Start menu |
| append | Add keyboard |
| setting | Initial settings |
| finish | Complete test |

---

## Reusable Components

| Component | Use |
|-----------|-----|
| IconNavigationLink | Navigation link with icon |
| ImageSlideshowView | Auto-rotating images |
| DraggableView | Drag to reorder |
| KeyboardPreview | Keyboard preview |
| DisclosuringList | Expandable list |

---

## Taigi Keyboard Application Suggestions

### Worth Referencing

| azooKey Design | Taigi Keyboard Application |
|----------------|---------------------------|
| Flick five-way input | Tone variations (a → á/à/â/ā/a̍) |
| UnifiedKeyModelProtocol | Unified key interface |
| State-driven rendering | POJ/TL switching |
| TabView + NavigationStack | Main App architecture |
| Onboarding flow | First launch guide |
| Search settings | Settings Tab |

### Not Needed

| azooKey Design | Reason |
|----------------|--------|
| CustardKit custom layouts | Taigi input needs are fixed |
| Layout editor UI | High development cost |
| Dual-track layout system | Focus on QWERTY first |

---

## SwiftUI Techniques

### MatchedGeometryEffect Animation

```swift
@Namespace private var namespace

.matchedGeometryEffect(id: "checkmark", in: namespace)
.animation(.easeIn(duration: 0.15), value: selectedIndex)
```

### TimelineView Carousel

```swift
TimelineView(.periodic(from: .now, by: 2.5)) { context in
    let index = Int(context.date.timeIntervalSince1970 / 2.5) % count
    Image(pictures[index])
}
```

### Custom ViewModifier

```swift
extension View {
    func focus(_ color: Color, focused: Bool) -> some View { }
    func onEnterBackground(perform: ...) -> some View { }
}
```

---

## Design Patterns

| Pattern | Implementation |
|---------|----------------|
| State Management | @StateObject + @EnvironmentObject |
| Enum-based Routing | Path Enum + NavigationStack |
| Composition | ViewBuilder + generic |
| Protocol-based Settings | BoolKeyboardSettingKey |

---

## azooKey Source Code Reference

| Directory | Content |
|-----------|---------|
| `KeyboardViews/` | Keyboard UI implementation |
| `KeyboardViews/View/UnifiedKey/` | Unified key system |
| `KeyboardViews/Custard/` | Built-in Flick layouts |
| `MainApp/` | Main application UI |
| `MainApp/Setting/` | Settings feature |
