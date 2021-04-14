```yaml
title: Node
```

# Node


```rust
use ockam::{Context, Result};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Stop the node as soon as it starts.
    ctx.stop().await
}
```

<div style="display: none; visibility: hidden;">
<a href="../02-worker">02. Worker</a>
</div>
