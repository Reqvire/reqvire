# Test Requirements

## Test Section

Here's a custom diagram that should be preserved:

```mermaid
graph TD;
    A[Custom Diagram] --> B[Should Not Be Removed];
    B --> C[This is User-Created];
```

---

### Test Requirement

This is a test requirement to verify diagram removal functionality.

#### Metadata
  * type: user-requirement

#### Relations
  * verifiedBy: [Test Verification](#test-verification)

---

### Test Verification

This verification tests the requirement.

#### Metadata
  * type: verification

#### Relations
  * verify: [Test Requirement](#test-requirement)

---

## Another Section
---

### Another Requirement

This is another test requirement.

#### Metadata
  * type: user-requirement

---