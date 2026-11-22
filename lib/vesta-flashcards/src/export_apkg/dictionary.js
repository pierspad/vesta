(function () {
  var language = __VESTA_LANGUAGE__;
  var sections = {en: 'English', ja: 'Japanese', zh: 'Chinese', ko: 'Korean',
    it: 'Italian', fr: 'French', de: 'German', es: 'Spanish', pt: 'Portuguese',
    ru: 'Russian', ar: 'Arabic', hi: 'Hindi', nl: 'Dutch', pl: 'Polish',
    tr: 'Turkish', uk: 'Ukrainian', el: 'Greek', he: 'Hebrew', th: 'Thai',
    vi: 'Vietnamese', id: 'Indonesian', sv: 'Swedish', fi: 'Finnish'};
  var section = sections[language.toLowerCase().split('-')[0]];
  function url(word) {
    return 'https://en.wiktionary.org/wiki/' + encodeURIComponent(word)
      + (section ? '#' + encodeURIComponent(section) : '');
  }
  var segmenter = null;
  try {
    if (language !== 'und' && typeof Intl !== 'undefined' && Intl.Segmenter)
      segmenter = new Intl.Segmenter(language, {granularity: 'word'});
  } catch (_) {}
  document.querySelectorAll('.expression').forEach(function (expression) {
    if (expression.dataset.vestaDictionary) return;
    expression.dataset.vestaDictionary = 'true';
    // Walk text nodes, preserving ruby, formatting, media and existing links.
    if (segmenter) {
      var walker = document.createTreeWalker(expression, NodeFilter.SHOW_TEXT);
      var nodes = [], node;
      while ((node = walker.nextNode())) {
        if (!node.parentElement.closest('a,script,style,rt,rp,button,textarea')) nodes.push(node);
      }
      nodes.forEach(function (text) {
        var fragment = document.createDocumentFragment();
        Array.from(segmenter.segment(text.nodeValue)).forEach(function (part) {
          if (!part.isWordLike) { fragment.appendChild(document.createTextNode(part.segment)); return; }
          var link = document.createElement('a');
          link.textContent = part.segment;
          link.href = url(part.segment);
          link.target = '_blank';
          link.rel = 'noopener noreferrer';
          link.title = 'Wiktionary (' + language + ')';
          link.style.cssText = 'color:inherit;text-decoration:underline dotted;text-underline-offset:0.15em';
          fragment.appendChild(link);
        });
        text.parentNode.replaceChild(fragment, text);
      });
    }
    // Manual selection also handles compounds and older Anki webviews.
    var lookup = document.createElement('a');
    lookup.textContent = 'Wiktionary ↗';
    lookup.target = '_blank';
    lookup.rel = 'noopener noreferrer';
    lookup.style.cssText = 'display:inline-block;font-size:0.65em;margin:0.5em';
    lookup.href = 'https://en.wiktionary.org/';
    lookup.title = 'Select text in the sentence, then open Wiktionary';
    lookup.addEventListener('click', function (event) {
      var selection = window.getSelection();
      var word = selection ? selection.toString().trim() : '';
      if (!word || !selection.anchorNode || !selection.focusNode
          || !expression.contains(selection.anchorNode) || !expression.contains(selection.focusNode)) {
        event.preventDefault();
        return;
      }
      lookup.href = url(word);
    });
    expression.insertAdjacentElement('afterend', lookup);
  });
})();
