# Device Adaptation

> **Type**: Feature
> **Keywords**: `Device`, `iPhone`, `iPad`, `ScreenSizeClass`
> **Related**: layout.md

---

## Summary

- Device classification + predefined constants (non-linear scaling)
- 4 size classes: phoneCompact / phoneRegular / phoneLarge / pad
- Candidate font size adjusts by class

---

## Screen Size Classification

| Class | Width condition | Representative devices |
|-------|-----------------|----------------------|
| `phoneCompact` | < 375pt | iPhone SE, mini |
| `phoneRegular` | 375-413pt | iPhone 14, 15 |
| `phoneLarge` | ≥ 414pt | iPhone Plus/Max |
| `pad` | iPad | iPad |

---

## Font Size Configuration

| Property | compact | regular | large | pad |
|----------|---------|---------|-------|-----|
| primaryFontSize | 20 | 21 | 20 | 23 |
| secondaryFontSize | 15 | 16 | 15 | 17 |

(`CandidateTheme` also derives `tpsPrimaryFontSize` / `tpsSecondaryFontSize` from these.)

---

## Affected Components

| Component | File |
|-----------|------|
| Candidate row | `CandidateView.swift` |
| Expanded grid | `ExpandedCandidateOverlay.swift` |

### Not Affected

- Key text: KeyboardKit handles automatically
- Callout: KeyboardKit standard style

---

## Test Devices

### Required Test Models

| Class | Device | Width |
|-------|--------|-------|
| phoneCompact | iPhone 13 mini | 375pt |
| phoneRegular | iPhone 15 | 393pt |
| phoneLarge | iPhone 15 Pro Max | 430pt |
| pad | iPad Pro 11" | 834pt |

---

## Screen Size Reference

| Device | Logical width | Class |
|--------|--------------|-------|
| iPhone SE (3rd) | 375pt | phoneRegular |
| iPhone 13 mini | 375pt | phoneCompact |
| iPhone 14/15 | 390-393pt | phoneRegular |
| iPhone Plus/Max | 428-430pt | phoneLarge |
