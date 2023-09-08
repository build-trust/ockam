function makeupProcessMatchingGroups() {
  var HIGHLIGHT_CLASS = "hll";
  function onMouseEnter(evt) {
    var groupId = evt.target.getAttribute("data-group-id");
    siblings = document.querySelectorAll("[data-group-id='" + groupId + "']");
    for (i = 0; i < siblings.length; ++i) {
      siblings[i].classList.add(HIGHLIGHT_CLASS);
    }
  }

  function onMouseLeave(evt) {
    var groupId = evt.target.getAttribute("data-group-id");
    siblings = document.querySelectorAll("[data-group-id='" + groupId + "']");
    for (i = 0; i < siblings.length; ++i) {
      siblings[i].classList.remove(HIGHLIGHT_CLASS);
    }
  }

  var delims = document.querySelectorAll("[data-group-id]");
  for(i=0; i < delims.length; i++) {
    var elem = delims[i];
    elem.addEventListener("mouseenter", onMouseEnter);
    elem.addEventListener("mouseleave", onMouseLeave);
  }
}

makeupProcessMatchingGroups();