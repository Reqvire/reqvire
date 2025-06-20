# Requirements

## Code Traecability
```mermaid
graph LR;
  %% Graph styling
  classDef requirement fill:#f9d6d6,stroke:#f55f5f,stroke-width:1px;
  classDef verification fill:#d6f9d6,stroke:#5fd75f,stroke-width:1px;
  classDef externalLink fill:#d0e0ff,stroke:#3080ff,stroke-width:1px;
  classDef default fill:#f5f5f5,stroke:#333333,stroke-width:1px;

  3db41d3530e56596["Validating Traceability Format"];
  class 3db41d3530e56596 requirement;
  click 3db41d3530e56596 "CodeTraecabilityRequirements.md#validating-traceability-format";
  9314736a59776478["Traceability Format"];
  class 9314736a59776478 requirement;
  click 9314736a59776478 "CodeTraecabilityRequirements.md#traceability-format";
  3db41d3530e56596 -->|refines| 9314736a59776478;
  be41c3d9e39abe6f["Dash style comment"];
  class be41c3d9e39abe6f requirement;
  click be41c3d9e39abe6f "CodeTraecabilityRequirements.md#dash-style-comment";
  f4c09ad7359812ff["Comment Style by File Extension"];
  class f4c09ad7359812ff requirement;
  click f4c09ad7359812ff "CodeTraecabilityRequirements.md#comment-style-by-file-extension";
  be41c3d9e39abe6f --o|contains| f4c09ad7359812ff;
  474c97b7d0029622["XML style comment"];
  class 474c97b7d0029622 requirement;
  click 474c97b7d0029622 "CodeTraecabilityRequirements.md#xml-style-comment";
  474c97b7d0029622 --o|contains| f4c09ad7359812ff;
  39fc946195c12956["SQL style comment"];
  class 39fc946195c12956 requirement;
  click 39fc946195c12956 "CodeTraecabilityRequirements.md#sql-style-comment";
  39fc946195c12956 --o|contains| f4c09ad7359812ff;
  a91fc474f292f1c9["CSS style comment"];
  class a91fc474f292f1c9 requirement;
  click a91fc474f292f1c9 "CodeTraecabilityRequirements.md#css-style-comment";
  a91fc474f292f1c9 --o|contains| f4c09ad7359812ff;
  26ca72b617aff229["../specifications/UserRequirements#Code Traceability"];
  class 26ca72b617aff229 requirement;
  click 26ca72b617aff229 "UserRequirements.md#code-traceability";
  9314736a59776478 -->|refines| 26ca72b617aff229;
  99c4cc438042d855["Slash style comment"];
  class 99c4cc438042d855 requirement;
  click 99c4cc438042d855 "CodeTraecabilityRequirements.md#slash-style-comment";
  99c4cc438042d855 --o|contains| f4c09ad7359812ff;
  7fdde6a23819450b["BAT style comment"];
  class 7fdde6a23819450b requirement;
  click 7fdde6a23819450b "CodeTraecabilityRequirements.md#bat-style-comment";
  7fdde6a23819450b --o|contains| f4c09ad7359812ff;
  f4c09ad7359812ff -->|refines| 9314736a59776478;
```
TODO:
 * this comes with config param to define code folder locations (special case as external foders but for the code) 
 * this is not implemented yet !!

---

### Traceability Format

When parsing a source file for traceability, the system shall identify and extract all `[reqvire::...]` markers along with their associated requirement element identifiers.

#### Details

Syntax used for `[reqvire::...]` markers:

```
[reqvire::<relation_type>: <element identifier>] START [reqvire::<relation_type>: <element idetifier>] END

```

Where:
- `<relation_type>` specifies the type of relation with only 2 types allowed:
  * satisfies
  * trace
- `<element identifier>` is the identifier of the requirement being traced.

#### Relations
  * refine: [../specifications/UserRequirements#Code Traceability](../specifications/UserRequirements.md#code-traceability)

---

### Validating Traceability Format


While processing traceability in code, the system shall ensure that each `[reqvire::...] START` tag has a corresponding `[reqvire::...] END` tag.

#### Relations
  * refine: [Traceability Format](#traceability-format)

---

### Comment Style by File Extension




The system shall use different comment style based of file extension of the code source file.

#### Relations
  * refine: [Traceability Format](#traceability-format)

---

### Slash style comment

When a source file has a `.c`, `.cpp`, `.cs`, `.java`, `.js`, or `.ts` extension, the system shall use `//` for single-line comments.

#### Details

```
// [reqvire::satisfies: Req1] START
void processSensorData() {
    // Implementation logic
}
// [reqvire::satisfies: Req1] END
```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---

### Dash style comment

When a source file has a `.py`, `.sh`, `.rb`, or `.yml` extension, the system shall use `#` for single-line comments.

#### Details

```
# [reqvire::satisfies: Req1] START
def process_sensor_data():
    pass  # Implementation logic
# [reqvire::satisfies: Req1] END
```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---

### XML style comment

When a source file has a `.html`, `.xml`, or `.xsl` extension, the system shall use `<!-- -->` for comments.

#### Details

```
<!-- [reqvire::satisfies: Req1] START -->
<div> UI Component </div>
<!-- [reqvire::satisfies: Req1] END -->

```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---

### CSS style comment

When a source file has a `.css` or `.scss` extension, the system shall use `/* */` for comments.

#### Details

```
/* [reqvire::satisfies: Req1] START */
.button { background-color: blue; }
/* [reqvire::satisfies: Req1] END */
```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---

### SQL style comment

When a source file has a `.sql` extension, the system shall use `--` for single-line comments.

```
-- [reqvire::satisfies: Req1] START
SELECT * FROM users;
-- [reqvire::satisfies: Req1] END
```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---

### BAT style comment

When a source file has a `.bat` or `.cmd` extension, the system shall use `REM` for comments.

#### Details

```
REM [reqvire::satisfies: Req1] START

echo Hello, World!

REM [reqvire::satisfies: Req1] END

```

#### Relations
  * containedBy: [Comment Style by File Extension](#comment-style-by-file-extension)

---