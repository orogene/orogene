/* eslint-disable */
/*!
 * vue-router v4.0.0-beta.7
 * (c) 2020 Eduardo San Martin Morote
 * @license MIT
 */
import {
  getCurrentInstance,
  warn as warn$1,
  inject,
  computed,
  unref,
  defineComponent,
  reactive,
  h,
  provide,
  ref,
  shallowRef,
  nextTick
} from 'vue';

const hasSymbol =
  typeof Symbol === 'function' && typeof Symbol.toStringTag === 'symbol';
const PolySymbol = (name) =>
  // vr = vue router
  hasSymbol ? Symbol('[vue-router]: ' + name) : '[vue-router]: ' + name;
// rvlm = Router View Location Matched
const matchedRouteKey = PolySymbol('router view location matched');
// rvd = Router View Depth
const viewDepthKey = PolySymbol('router view depth');
// r = router
const routerKey = PolySymbol('router');
// rt = route location
const routeLocationKey = PolySymbol('route location');

const isBrowser = typeof window !== 'undefined';

function isESModule(obj) {
  return obj.__esModule || (hasSymbol && obj[Symbol.toStringTag] === 'Module');
}
const assign = Object.assign;
function applyToParams(fn, params) {
  const newParams = {};
  for (const key in params) {
    const value = params[key];
    newParams[key] = Array.isArray(value) ? value.map(fn) : fn(value);
  }
  return newParams;
}
let noop = () => {};

function warn(msg) {
  // avoid using ...args as it breaks in older Edge builds
  const args = Array.from(arguments).slice(1);
  console.warn.apply(console, ['[Vue Router warn]: ' + msg].concat(args));
}

const TRAILING_SLASH_RE = /\/$/;
const removeTrailingSlash = (path) => path.replace(TRAILING_SLASH_RE, '');
/**
 * Transforms an URI into a normalized history location
 *
 * @param parseQuery
 * @param location - URI to normalize
 * @param currentLocation - current absolute location. Allows resolving relative
 * paths. Must start with `/`. Defaults to `/`
 * @returns a normalized history location
 */
function parseURL(parseQuery, location, currentLocation = '/') {
  let path,
    query = {},
    searchString = '',
    hash = '';
  // Could use URL and URLSearchParams but IE 11 doesn't support it
  const searchPos = location.indexOf('?');
  const hashPos = location.indexOf('#', searchPos > -1 ? searchPos : 0);
  if (searchPos > -1) {
    path = location.slice(0, searchPos);
    searchString = location.slice(
      searchPos + 1,
      hashPos > -1 ? hashPos : location.length
    );
    query = parseQuery(searchString);
  }
  if (hashPos > -1) {
    path = path || location.slice(0, hashPos);
    // keep the # character
    hash = location.slice(hashPos, location.length);
  }
  // no search and no query
  path = resolveRelativePath(path != null ? path : location, currentLocation);
  // empty path means a relative query or hash `?foo=f`, `#thing`
  return {
    fullPath: path + (searchString && '?') + searchString + hash,
    path,
    query,
    hash
  };
}
/**
 * Stringifies a URL object
 *
 * @param stringifyQuery
 * @param location
 */
function stringifyURL(stringifyQuery, location) {
  let query = location.query ? stringifyQuery(location.query) : '';
  return location.path + (query && '?') + query + (location.hash || '');
}
/**
 * Strips off the base from the beginning of a location.pathname in a non
 * case-sensitive way.
 *
 * @param pathname - location.pathname
 * @param base - base to strip off
 */
function stripBase(pathname, base) {
  // no base or base is not found at the beginning
  if (!base || pathname.toLowerCase().indexOf(base.toLowerCase()))
    return pathname;
  return pathname.slice(base.length) || '/';
}
/**
 * Checks if two RouteLocation are equal. This means that both locations are
 * pointing towards the same {@link RouteRecord} and that all `params`, `query`
 * parameters and `hash` are the same
 *
 * @param a first {@link RouteLocation}
 * @param b second {@link RouteLocation}
 */
function isSameRouteLocation(stringifyQuery, a, b) {
  let aLastIndex = a.matched.length - 1;
  let bLastIndex = b.matched.length - 1;
  return (
    aLastIndex > -1 &&
    aLastIndex === bLastIndex &&
    isSameRouteRecord(a.matched[aLastIndex], b.matched[bLastIndex]) &&
    isSameRouteLocationParams(a.params, b.params) &&
    stringifyQuery(a.query) === stringifyQuery(b.query) &&
    a.hash === b.hash
  );
}
/**
 * Check if two `RouteRecords` are equal. Takes into account aliases: they are
 * considered equal to the `RouteRecord` they are aliasing.
 *
 * @param a first {@link RouteRecord}
 * @param b second {@link RouteRecord}
 */
function isSameRouteRecord(a, b) {
  // since the original record has an undefined value for aliasOf
  // but all aliases point to the original record, this will always compare
  // the original record
  return (a.aliasOf || a) === (b.aliasOf || b);
}
function isSameRouteLocationParams(a, b) {
  if (Object.keys(a).length !== Object.keys(b).length) return false;
  for (let key in a) {
    if (!isSameRouteLocationParamsValue(a[key], b[key])) return false;
  }
  return true;
}
function isSameRouteLocationParamsValue(a, b) {
  return Array.isArray(a)
    ? isEquivalentArray(a, b)
    : Array.isArray(b)
    ? isEquivalentArray(b, a)
    : a === b;
}
/**
 * Check if two arrays are the same or if an array with one single entry is the
 * same as another primitive value. Used to check query and parameters
 *
 * @param a - array of values
 * @param b - array of values or a single value
 */
function isEquivalentArray(a, b) {
  return Array.isArray(b)
    ? a.length === b.length && a.every((value, i) => value === b[i])
    : a.length === 1 && a[0] === b;
}
/**
 * Resolves a relative path that starts with `.`.
 *
 * @param to - path location we are resolving
 * @param from - currentLocation.path, should start with `/`
 */
function resolveRelativePath(to, from) {
  if (to.startsWith('/')) return to;
  if (!from.startsWith('/')) {
    warn(
      `Cannot resolve a relative location without an absolute path. Trying to resolve "${to}" from "${from}". It should look like "/${from}".`
    );
    return to;
  }
  if (!to) return from;
  const fromSegments = from.split('/');
  const toSegments = to.split('/');
  let position = fromSegments.length - 1;
  let toPosition;
  let segment;
  for (toPosition = 0; toPosition < toSegments.length; toPosition++) {
    segment = toSegments[toPosition];
    // can't go below zero
    if (position === 1 || segment === '.') continue;
    if (segment === '..') position--;
    // found something that is not relative path
    else break;
  }
  return (
    fromSegments.slice(0, position).join('/') +
    '/' +
    toSegments
      .slice(toPosition - (toPosition === toSegments.length ? 1 : 0))
      .join('/')
  );
}

var NavigationType;
(function (NavigationType) {
  NavigationType['pop'] = 'pop';
  NavigationType['push'] = 'push';
})(NavigationType || (NavigationType = {}));
var NavigationDirection;
(function (NavigationDirection) {
  NavigationDirection['back'] = 'back';
  NavigationDirection['forward'] = 'forward';
  NavigationDirection['unknown'] = '';
})(NavigationDirection || (NavigationDirection = {}));
/**
 * Starting location for Histories
 */
const START = '';
// Generic utils
/**
 * Normalizes a base by removing any trailing slash and reading the base tag if
 * present.
 *
 * @param base - base to normalize
 */
function normalizeBase(base) {
  if (!base) {
    if (isBrowser) {
      // respect <base> tag
      const baseEl = document.querySelector('base');
      base = (baseEl && baseEl.getAttribute('href')) || '/';
      // strip full URL origin
      base = base.replace(/^\w+:\/\/[^\/]+/, '');
    } else {
      base = '/';
    }
  }
  // ensure leading slash when it was removed by the regex above avoid leading
  // slash with hash because the file could be read from the disk like file://
  // and the leading slash would cause problems
  if (base[0] !== '/' && base[0] !== '#') base = '/' + base;
  // remove the trailing slash so all other method can just do `base + fullPath`
  // to build an href
  return removeTrailingSlash(base);
}
// remove any character before the hash
const BEFORE_HASH_RE = /^[^#]+#/;
function createHref(base, location) {
  return base.replace(BEFORE_HASH_RE, '#') + location;
}

function getElementPosition(el, offset) {
  const docRect = document.documentElement.getBoundingClientRect();
  const elRect = el.getBoundingClientRect();
  return {
    behavior: offset.behavior,
    left: elRect.left - docRect.left - (offset.left || 0),
    top: elRect.top - docRect.top - (offset.top || 0)
  };
}
const computeScrollPosition = () => ({
  left: window.pageXOffset,
  top: window.pageYOffset
});
function scrollToPosition(position) {
  let scrollToOptions;
  if ('el' in position) {
    let positionEl = position.el;
    const isIdSelector =
      typeof positionEl === 'string' && positionEl.startsWith('#');
    /**
     * `id`s can accept pretty much any characters, including CSS combinators
     * like `>` or `~`. It's still possible to retrieve elements using
     * `document.getElementById('~')` but it needs to be escaped when using
     * `document.querySelector('#\\~')` for it to be valid. The only
     * requirements for `id`s are them to be unique on the page and to not be
     * empty (`id=""`). Because of that, when passing an id selector, it should
     * be properly escaped for it to work with `querySelector`. We could check
     * for the id selector to be simple (no CSS combinators `+ >~`) but that
     * would make things inconsistent since they are valid characters for an
     * `id` but would need to be escaped when using `querySelector`, breaking
     * their usage and ending up in no selector returned. Selectors need to be
     * escaped:
     *
     * - `#1-thing` becomes `#\31 -thing`
     * - `#with~symbols` becomes `#with\\~symbols`
     *
     * - More information about  the topic can be found at
     *   https://mathiasbynens.be/notes/html5-id-class.
     * - Practical example: https://mathiasbynens.be/demo/html5-id
     */
    if (typeof position.el === 'string') {
      if (!isIdSelector || !document.getElementById(position.el.slice(1))) {
        try {
          let foundEl = document.querySelector(position.el);
          if (isIdSelector && foundEl) {
            warn(
              `The selector "${position.el}" should be passed as "el: document.querySelector('${position.el}')" because it starts with "#".`
            );
            // return to avoid other warnings
            return;
          }
        } catch (err) {
          warn(
            `The selector "${position.el}" is invalid. If you are using an id selector, make sure to escape it. You can find more information about escaping characters in selectors at https://mathiasbynens.be/notes/css-escapes or use CSS.escape (https://developer.mozilla.org/en-US/docs/Web/API/CSS/escape).`
          );
          // return to avoid other warnings
          return;
        }
      }
    }
    const el =
      typeof positionEl === 'string'
        ? isIdSelector
          ? document.getElementById(positionEl.slice(1))
          : document.querySelector(positionEl)
        : positionEl;
    if (!el) {
      warn(`Couldn't find element using selector "${position.el}"`);
      return;
    }
    scrollToOptions = getElementPosition(el, position);
  } else {
    scrollToOptions = position;
  }
  if ('scrollBehavior' in document.documentElement.style)
    window.scrollTo(scrollToOptions);
  else {
    window.scrollTo(
      scrollToOptions.left != null ? scrollToOptions.left : window.pageXOffset,
      scrollToOptions.top != null ? scrollToOptions.top : window.pageYOffset
    );
  }
}
function getScrollKey(path, delta) {
  const position = history.state ? history.state.position - delta : -1;
  return position + path;
}
const scrollPositions = new Map();
function saveScrollPosition(key, scrollPosition) {
  scrollPositions.set(key, scrollPosition);
}
function getSavedScrollPosition(key) {
  const scroll = scrollPositions.get(key);
  // consume it so it's not used again
  scrollPositions.delete(key);
  return scroll;
}
// TODO: RFC about how to save scroll position
/**
 * ScrollBehavior instance used by the router to compute and restore the scroll
 * position when navigating.
 */
// export interface ScrollHandler<ScrollPositionEntry extends HistoryStateValue, ScrollPosition extends ScrollPositionEntry> {
//   // returns a scroll position that can be saved in history
//   compute(): ScrollPositionEntry
//   // can take an extended ScrollPositionEntry
//   scroll(position: ScrollPosition): void
// }
// export const scrollHandler: ScrollHandler<ScrollPosition> = {
//   compute: computeScroll,
//   scroll: scrollToPosition,
// }

let createBaseLocation = () => location.protocol + '//' + location.host;
/**
 * Creates a normalized history location from a window.location object
 * @param location
 */
function createCurrentLocation(base, location) {
  const { pathname, search, hash } = location;
  // allows hash based url
  const hashPos = base.indexOf('#');
  if (hashPos > -1) {
    // prepend the starting slash to hash so the url starts with /#
    let pathFromHash = hash.slice(1);
    if (pathFromHash[0] !== '/') pathFromHash = '/' + pathFromHash;
    return stripBase(pathFromHash, '');
  }
  const path = stripBase(pathname, base);
  return path + search + hash;
}
function useHistoryListeners(base, historyState, currentLocation, replace) {
  let listeners = [];
  let teardowns = [];
  // TODO: should it be a stack? a Dict. Check if the popstate listener
  // can trigger twice
  let pauseState = null;
  const popStateHandler = ({ state }) => {
    const to = createCurrentLocation(base, location);
    const from = currentLocation.value;
    const fromState = historyState.value;
    let delta = 0;
    if (state) {
      currentLocation.value = to;
      historyState.value = state;
      // ignore the popstate and reset the pauseState
      if (pauseState && pauseState === from) {
        pauseState = null;
        return;
      }
      delta = fromState ? state.position - fromState.position : 0;
    } else {
      replace(to);
    }
    // console.log({ deltaFromCurrent })
    // Here we could also revert the navigation by calling history.go(-delta)
    // this listener will have to be adapted to not trigger again and to wait for the url
    // to be updated before triggering the listeners. Some kind of validation function would also
    // need to be passed to the listeners so the navigation can be accepted
    // call all listeners
    listeners.forEach((listener) => {
      listener(currentLocation.value, from, {
        delta,
        type: NavigationType.pop,
        direction: delta
          ? delta > 0
            ? NavigationDirection.forward
            : NavigationDirection.back
          : NavigationDirection.unknown
      });
    });
  };
  function pauseListeners() {
    pauseState = currentLocation.value;
  }
  function listen(callback) {
    // setup the listener and prepare teardown callbacks
    listeners.push(callback);
    const teardown = () => {
      const index = listeners.indexOf(callback);
      if (index > -1) listeners.splice(index, 1);
    };
    teardowns.push(teardown);
    return teardown;
  }
  function beforeUnloadListener() {
    const { history } = window;
    if (!history.state) return;
    history.replaceState(
      assign({}, history.state, { scroll: computeScrollPosition() }),
      ''
    );
  }
  function destroy() {
    for (const teardown of teardowns) teardown();
    teardowns = [];
    window.removeEventListener('popstate', popStateHandler);
    window.removeEventListener('beforeunload', beforeUnloadListener);
  }
  // setup the listeners and prepare teardown callbacks
  window.addEventListener('popstate', popStateHandler);
  window.addEventListener('beforeunload', beforeUnloadListener);
  return {
    pauseListeners,
    listen,
    destroy
  };
}
/**
 * Creates a state object
 */
function buildState(
  back,
  current,
  forward,
  replaced = false,
  computeScroll = false
) {
  return {
    back,
    current,
    forward,
    replaced,
    position: window.history.length,
    scroll: computeScroll ? computeScrollPosition() : null
  };
}
function useHistoryStateNavigation(base) {
  const { history, location } = window;
  // private variables
  let currentLocation = {
    value: createCurrentLocation(base, location)
  };
  let historyState = { value: history.state };
  // build current history entry as this is a fresh navigation
  if (!historyState.value) {
    changeLocation(
      currentLocation.value,
      {
        back: null,
        current: currentLocation.value,
        forward: null,
        // the length is off by one, we need to decrease it
        position: history.length - 1,
        replaced: true,
        // don't add a scroll as the user may have an anchor and we want
        // scrollBehavior to be triggered without a saved position
        scroll: null
      },
      true
    );
  }
  function changeLocation(to, state, replace) {
    const url =
      createBaseLocation() +
      // preserve any existing query when base has a hash
      (base.indexOf('#') > -1 && location.search
        ? location.pathname + location.search + '#'
        : base) +
      to;
    try {
      // BROWSER QUIRK
      // NOTE: Safari throws a SecurityError when calling this function 100 times in 30 seconds
      history[replace ? 'replaceState' : 'pushState'](state, '', url);
      historyState.value = state;
    } catch (err) {
      warn('Error with push/replace State', err);
      // Force the navigation, this also resets the call count
      location[replace ? 'replace' : 'assign'](url);
    }
  }
  function replace(to, data) {
    const state = assign(
      {},
      history.state,
      buildState(
        historyState.value.back,
        // keep back and forward entries but override current position
        to,
        historyState.value.forward,
        true
      ),
      data,
      { position: historyState.value.position }
    );
    changeLocation(to, state, true);
    currentLocation.value = to;
  }
  function push(to, data) {
    // Add to current entry the information of where we are going
    // as well as saving the current position
    const currentState = assign({}, history.state, {
      forward: to,
      scroll: computeScrollPosition()
    });
    changeLocation(currentState.current, currentState, true);
    const state = assign(
      {},
      buildState(currentLocation.value, to, null),
      {
        position: currentState.position + 1
      },
      data
    );
    changeLocation(to, state, false);
    currentLocation.value = to;
  }
  return {
    location: currentLocation,
    state: historyState,
    push,
    replace
  };
}
function createWebHistory(base) {
  base = normalizeBase(base);
  const historyNavigation = useHistoryStateNavigation(base);
  const historyListeners = useHistoryListeners(
    base,
    historyNavigation.state,
    historyNavigation.location,
    historyNavigation.replace
  );
  function go(delta, triggerListeners = true) {
    if (!triggerListeners) historyListeners.pauseListeners();
    history.go(delta);
  }
  const routerHistory = assign(
    {
      // it's overridden right after
      location: '',
      base,
      go,
      createHref: createHref.bind(null, base)
    },
    historyNavigation,
    historyListeners
  );
  Object.defineProperty(routerHistory, 'location', {
    get: () => historyNavigation.location.value
  });
  Object.defineProperty(routerHistory, 'state', {
    get: () => historyNavigation.state.value
  });
  return routerHistory;
}

// TODO: verify base is working for SSR
/**
 * Creates a in-memory based history. The main purpose of this history is to handle SSR. It starts in a special location that is nowhere.
 * It's up to the user to replace that location with the starter location.
 * @param base - Base applied to all urls, defaults to '/'
 * @returns a history object that can be passed to the router constructor
 */
function createMemoryHistory(base = '') {
  let listeners = [];
  let queue = [START];
  let position = 0;
  function setLocation(location) {
    position++;
    if (position === queue.length) {
      // we are at the end, we can simply append a new entry
      queue.push(location);
    } else {
      // we are in the middle, we remove everything from here in the queue
      queue.splice(position);
      queue.push(location);
    }
  }
  function triggerListeners(to, from, { direction, delta }) {
    const info = {
      direction,
      delta,
      type: NavigationType.pop
    };
    for (let callback of listeners) {
      callback(to, from, info);
    }
  }
  const routerHistory = {
    // rewritten by Object.defineProperty
    location: START,
    state: {},
    base,
    createHref: createHref.bind(null, base),
    replace(to) {
      // remove current entry and decrement position
      queue.splice(position--, 1);
      setLocation(to);
    },
    push(to, data) {
      setLocation(to);
    },
    listen(callback) {
      listeners.push(callback);
      return () => {
        const index = listeners.indexOf(callback);
        if (index > -1) listeners.splice(index, 1);
      };
    },
    destroy() {
      listeners = [];
    },
    go(delta, shouldTrigger = true) {
      const from = this.location;
      const direction =
        // we are considering delta === 0 going forward, but in abstract mode
        // using 0 for the delta doesn't make sense like it does in html5 where
        // it reloads the page
        delta < 0 ? NavigationDirection.back : NavigationDirection.forward;
      position = Math.max(0, Math.min(position + delta, queue.length - 1));
      if (shouldTrigger) {
        triggerListeners(this.location, from, {
          direction,
          delta
        });
      }
    }
  };
  Object.defineProperty(routerHistory, 'location', {
    get: () => queue[position]
  });
  return routerHistory;
}

/**
 * Creates a hash history.
 *
 * @param base - optional base to provide. Defaults to `location.pathname` or
 * `/` if at root. If there is a `base` tag in the `head`, its value will be
 * **ignored**.
 *
 * @example
 * ```js
 * // at https://example.com/folder
 * createWebHashHistory() // gives a url of `https://example.com/folder#`
 * createWebHashHistory('/folder/') // gives a url of `https://example.com/folder/#`
 * // if the `#` is provided in the base, it won't be added by `createWebHashHistory`
 * createWebHashHistory('/folder/#/app/') // gives a url of `https://example.com/folder/#/app/`
 * // you should avoid doing this because it changes the original url and breaks copying urls
 * createWebHashHistory('/other-folder/') // gives a url of `https://example.com/other-folder/#`
 *
 * // at file:///usr/etc/folder/index.html
 * // for locations with no `host`, the base is ignored
 * createWebHashHistory('/iAmIgnored') // gives a url of `file:///usr/etc/folder/index.html#`
 * ```
 */
function createWebHashHistory(base) {
  // Make sure this implementation is fine in terms of encoding, specially for IE11
  // for `file://`, directly use the pathname and ignore the base
  // location.pathname contains an initial `/` even at the root: `https://example.com`
  base = location.host ? base || location.pathname : location.pathname;
  // allow the user to provide a `#` in the middle: `/base/#/app`
  if (base.indexOf('#') < 0) base += '#';
  if (!base.endsWith('#/') && !base.endsWith('#')) {
    warn(
      `A hash base must end with a "#":\n"${base}" should be "${base.replace(
        /#.*$/,
        '#'
      )}".`
    );
  }
  return createWebHistory(base);
}

function isRouteLocation(route) {
  return typeof route === 'string' || (route && typeof route === 'object');
}
function isRouteName(name) {
  return typeof name === 'string' || typeof name === 'symbol';
}

const START_LOCATION_NORMALIZED = {
  path: '/',
  name: undefined,
  params: {},
  query: {},
  hash: '',
  fullPath: '/',
  matched: [],
  meta: {},
  redirectedFrom: undefined
};

const NavigationFailureSymbol = PolySymbol('navigation failure');
var NavigationFailureType;
(function (NavigationFailureType) {
  NavigationFailureType[(NavigationFailureType['aborted'] = 4)] = 'aborted';
  NavigationFailureType[(NavigationFailureType['cancelled'] = 8)] = 'cancelled';
  NavigationFailureType[(NavigationFailureType['duplicated'] = 16)] =
    'duplicated';
})(NavigationFailureType || (NavigationFailureType = {}));
// DEV only debug messages
const ErrorTypeMessages = {
  [1 /* MATCHER_NOT_FOUND */]({ location, currentLocation }) {
    return `No match for\n ${JSON.stringify(location)}${
      currentLocation
        ? '\nwhile being at\n' + JSON.stringify(currentLocation)
        : ''
    }`;
  },
  [2 /* NAVIGATION_GUARD_REDIRECT */]({ from, to }) {
    return `Redirected from "${from.fullPath}" to "${stringifyRoute(
      to
    )}" via a navigation guard.`;
  },
  [4 /* NAVIGATION_ABORTED */]({ from, to }) {
    return `Navigation aborted from "${from.fullPath}" to "${to.fullPath}" via a navigation guard.`;
  },
  [8 /* NAVIGATION_CANCELLED */]({ from, to }) {
    return `Navigation cancelled from "${from.fullPath}" to "${to.fullPath}" with a new navigation.`;
  },
  [16 /* NAVIGATION_DUPLICATED */]({ from, to }) {
    return `Avoided redundant navigation to current location: "${from.fullPath}".`;
  }
};
function createRouterError(type, params) {
  {
    return assign(
      new Error(ErrorTypeMessages[type](params)),
      {
        type,
        [NavigationFailureSymbol]: true
      },
      params
    );
  }
}
function isNavigationFailure(error, type) {
  return (
    error instanceof Error &&
    NavigationFailureSymbol in error &&
    (type == null || !!(error.type & type))
  );
}
const propertiesToLog = ['params', 'query', 'hash'];
function stringifyRoute(to) {
  if (typeof to === 'string') return to;
  if ('path' in to) return to.path;
  const location = {};
  for (const key of propertiesToLog) {
    if (key in to) location[key] = to[key];
  }
  return JSON.stringify(location, null, 2);
}

// default pattern for a param: non greedy everything but /
const BASE_PARAM_PATTERN = '[^/]+?';
const BASE_PATH_PARSER_OPTIONS = {
  sensitive: false,
  strict: false,
  start: true,
  end: true
};
// Special Regex characters that must be escaped in static tokens
const REGEX_CHARS_RE = /[.+*?^${}()[\]/\\]/g;
/**
 * Creates a path parser from an array of Segments (a segment is an array of Tokens)
 *
 * @param segments - array of segments returned by tokenizePath
 * @param extraOptions - optional options for the regexp
 * @returns a PathParser
 */
function tokensToParser(segments, extraOptions) {
  const options = assign({}, BASE_PATH_PARSER_OPTIONS, extraOptions);
  // the amount of scores is the same as the length of segments except for the root segment "/"
  let score = [];
  // the regexp as a string
  let pattern = options.start ? '^' : '';
  // extracted keys
  const keys = [];
  for (const segment of segments) {
    // the root segment needs special treatment
    const segmentScores = segment.length ? [] : [90 /* Root */];
    // allow trailing slash
    if (options.strict && !segment.length) pattern += '/';
    for (let tokenIndex = 0; tokenIndex < segment.length; tokenIndex++) {
      const token = segment[tokenIndex];
      // resets the score if we are inside a sub segment /:a-other-:b
      let subSegmentScore =
        40 /* Segment */ +
        (options.sensitive ? 0.25 /* BonusCaseSensitive */ : 0);
      if (token.type === 0 /* Static */) {
        // prepend the slash if we are starting a new segment
        if (!tokenIndex) pattern += '/';
        pattern += token.value.replace(REGEX_CHARS_RE, '\\$&');
        subSegmentScore += 40 /* Static */;
      } else if (token.type === 1 /* Param */) {
        const { value, repeatable, optional, regexp } = token;
        keys.push({
          name: value,
          repeatable,
          optional
        });
        const re = regexp ? regexp : BASE_PARAM_PATTERN;
        // the user provided a custom regexp /:id(\\d+)
        if (re !== BASE_PARAM_PATTERN) {
          subSegmentScore += 10 /* BonusCustomRegExp */;
          // make sure the regexp is valid before using it
          try {
            new RegExp(`(${re})`);
          } catch (err) {
            throw new Error(
              `Invalid custom RegExp for param "${value}" (${re}): ` +
                err.message
            );
          }
        }
        // when we repeat we must take care of the repeating leading slash
        let subPattern = repeatable ? `((?:${re})(?:/(?:${re}))*)` : `(${re})`;
        // prepend the slash if we are starting a new segment
        if (!tokenIndex)
          subPattern = optional ? `(?:/${subPattern})` : '/' + subPattern;
        if (optional) subPattern += '?';
        pattern += subPattern;
        subSegmentScore += 20 /* Dynamic */;
        if (optional) subSegmentScore += -8 /* BonusOptional */;
        if (repeatable) subSegmentScore += -20 /* BonusRepeatable */;
        if (re === '.*') subSegmentScore += -50 /* BonusWildcard */;
      }
      segmentScores.push(subSegmentScore);
    }
    // an empty array like /home/ -> [[{home}], []]
    // if (!segment.length) pattern += '/'
    score.push(segmentScores);
  }
  // only apply the strict bonus to the last score
  if (options.strict && options.end) {
    const i = score.length - 1;
    score[i][score[i].length - 1] += 0.7000000000000001 /* BonusStrict */;
  }
  // TODO: dev only warn double trailing slash
  if (!options.strict) pattern += '/?';
  if (options.end) pattern += '$';
  // allow paths like /dynamic to only match dynamic or dynamic/... but not dynamic_something_else
  else if (options.strict) pattern += '(?:/|$)';
  const re = new RegExp(pattern, options.sensitive ? '' : 'i');
  function parse(path) {
    const match = path.match(re);
    const params = {};
    if (!match) return null;
    for (let i = 1; i < match.length; i++) {
      const value = match[i] || '';
      const key = keys[i - 1];
      params[key.name] = value && key.repeatable ? value.split('/') : value;
    }
    return params;
  }
  function stringify(params) {
    let path = '';
    // for optional parameters to allow to be empty
    let avoidDuplicatedSlash = false;
    for (const segment of segments) {
      if (!avoidDuplicatedSlash || !path.endsWith('/')) path += '/';
      avoidDuplicatedSlash = false;
      for (const token of segment) {
        if (token.type === 0 /* Static */) {
          path += token.value;
        } else if (token.type === 1 /* Param */) {
          const { value, repeatable, optional } = token;
          const param = value in params ? params[value] : '';
          if (Array.isArray(param) && !repeatable)
            throw new Error(
              `Provided param "${value}" is an array but it is not repeatable (* or + modifiers)`
            );
          const text = Array.isArray(param) ? param.join('/') : param;
          if (!text) {
            if (optional) {
              // remove the last slash as we could be at the end
              if (path.endsWith('/')) path = path.slice(0, -1);
              // do not append a slash on the next iteration
              else avoidDuplicatedSlash = true;
            } else throw new Error(`Missing required param "${value}"`);
          }
          path += text;
        }
      }
    }
    return path;
  }
  return {
    re,
    score,
    keys,
    parse,
    stringify
  };
}
/**
 * Compares an array of numbers as used in PathParser.score and returns a
 * number. This function can be used to `sort` an array
 * @param a - first array of numbers
 * @param b - second array of numbers
 * @returns 0 if both are equal, < 0 if a should be sorted first, > 0 if b
 * should be sorted first
 */
function compareScoreArray(a, b) {
  let i = 0;
  while (i < a.length && i < b.length) {
    const diff = b[i] - a[i];
    // only keep going if diff === 0
    if (diff) return diff;
    i++;
  }
  // if the last subsegment was Static, the shorter segments should be sorted first
  // otherwise sort the longest segment first
  if (a.length < b.length) {
    return a.length === 1 && a[0] === 40 /* Static */ + 40 /* Segment */
      ? -1
      : 1;
  } else if (a.length > b.length) {
    return b.length === 1 && b[0] === 40 /* Static */ + 40 /* Segment */
      ? 1
      : -1;
  }
  return 0;
}
/**
 * Compare function that can be used with `sort` to sort an array of PathParser
 * @param a - first PathParser
 * @param b - second PathParser
 * @returns 0 if both are equal, < 0 if a should be sorted first, > 0 if b
 */
function comparePathParserScore(a, b) {
  let i = 0;
  const aScore = a.score;
  const bScore = b.score;
  while (i < aScore.length && i < bScore.length) {
    const comp = compareScoreArray(aScore[i], bScore[i]);
    // do not return if both are equal
    if (comp) return comp;
    i++;
  }
  // if a and b share the same score entries but b has more, sort b first
  return bScore.length - aScore.length;
  // this is the ternary version
  // return aScore.length < bScore.length
  //   ? 1
  //   : aScore.length > bScore.length
  //   ? -1
  //   : 0
}

const ROOT_TOKEN = {
  type: 0 /* Static */,
  value: ''
};
const VALID_PARAM_RE = /[a-zA-Z0-9_]/;
// After some profiling, the cache seems to be unnecessary because tokenizePath
// (the slowest part of adding a route) is very fast
// const tokenCache = new Map<string, Token[][]>()
function tokenizePath(path) {
  if (!path) return [[]];
  if (path === '/') return [[ROOT_TOKEN]];
  // remove the leading slash
  if (path[0] !== '/') throw new Error('A non-empty path must start with "/"');
  // if (tokenCache.has(path)) return tokenCache.get(path)!
  function crash(message) {
    throw new Error(`ERR (${state})/"${buffer}": ${message}`);
  }
  let state = 0; /* Static */
  let previousState = state;
  const tokens = [];
  // the segment will always be valid because we get into the initial state
  // with the leading /
  let segment;
  function finalizeSegment() {
    if (segment) tokens.push(segment);
    segment = [];
  }
  // index on the path
  let i = 0;
  // char at index
  let char;
  // buffer of the value read
  let buffer = '';
  // custom regexp for a param
  let customRe = '';
  function consumeBuffer() {
    if (!buffer) return;
    if (state === 0 /* Static */) {
      segment.push({
        type: 0 /* Static */,
        value: buffer
      });
    } else if (
      state === 1 /* Param */ ||
      state === 2 /* ParamRegExp */ ||
      state === 3 /* ParamRegExpEnd */
    ) {
      if (segment.length > 1 && (char === '*' || char === '+'))
        crash(
          `A repeatable param (${buffer}) must be alone in its segment. eg: '/:ids+.`
        );
      segment.push({
        type: 1 /* Param */,
        value: buffer,
        regexp: customRe,
        repeatable: char === '*' || char === '+',
        optional: char === '*' || char === '?'
      });
    } else {
      crash('Invalid state to consume buffer');
    }
    buffer = '';
  }
  function addCharToBuffer() {
    buffer += char;
  }
  while (i < path.length) {
    char = path[i++];
    if (char === '\\' && state !== 2 /* ParamRegExp */) {
      previousState = state;
      state = 4 /* EscapeNext */;
      continue;
    }
    switch (state) {
      case 0 /* Static */:
        if (char === '/') {
          if (buffer) {
            consumeBuffer();
          }
          finalizeSegment();
        } else if (char === ':') {
          consumeBuffer();
          state = 1 /* Param */;
        } else {
          addCharToBuffer();
        }
        break;
      case 4 /* EscapeNext */:
        addCharToBuffer();
        state = previousState;
        break;
      case 1 /* Param */:
        if (char === '(') {
          state = 2 /* ParamRegExp */;
          customRe = '';
        } else if (VALID_PARAM_RE.test(char)) {
          addCharToBuffer();
        } else {
          consumeBuffer();
          state = 0 /* Static */;
          // go back one character if we were not modifying
          if (char !== '*' && char !== '?' && char !== '+') i--;
        }
        break;
      case 2 /* ParamRegExp */:
        if (char === ')') {
          // handle the escaped )
          if (customRe[customRe.length - 1] == '\\')
            customRe = customRe.slice(0, -1) + char;
          else state = 3 /* ParamRegExpEnd */;
        } else {
          customRe += char;
        }
        break;
      case 3 /* ParamRegExpEnd */:
        // same as finalizing a param
        consumeBuffer();
        state = 0 /* Static */;
        // go back one character if we were not modifying
        if (char !== '*' && char !== '?' && char !== '+') i--;
        break;
      default:
        crash('Unknown state');
        break;
    }
  }
  if (state === 2 /* ParamRegExp */)
    crash(`Unfinished custom RegExp for param "${buffer}"`);
  consumeBuffer();
  finalizeSegment();
  // tokenCache.set(path, tokens)
  return tokens;
}

function createRouteRecordMatcher(record, parent, options) {
  const parser = tokensToParser(tokenizePath(record.path), options);
  // warn against params with the same name
  {
    const existingKeys = new Set();
    for (const key of parser.keys) {
      if (existingKeys.has(key.name))
        warn(
          `Found duplicated params with name "${key.name}" for path "${record.path}". Only the last one will be available on "$route.params".`
        );
      existingKeys.add(key.name);
    }
  }
  const matcher = assign(parser, {
    record,
    parent,
    // these needs to be populated by the parent
    children: [],
    alias: []
  });
  if (parent) {
    // both are aliases or both are not aliases
    // we don't want to mix them because the order is used when
    // passing originalRecord in Matcher.addRoute
    if (!matcher.record.aliasOf === !parent.record.aliasOf)
      parent.children.push(matcher);
  }
  return matcher;
}

/**
 * Creates a Router Matcher.
 *
 * @internal
 * @param routes - array of initial routes
 * @param globalOptions - global route options
 */
function createRouterMatcher(routes, globalOptions) {
  // normalized ordered array of matchers
  const matchers = [];
  const matcherMap = new Map();
  globalOptions = mergeOptions(
    { strict: false, end: true, sensitive: false },
    globalOptions
  );
  function getRecordMatcher(name) {
    return matcherMap.get(name);
  }
  function addRoute(record, parent, originalRecord) {
    // used later on to remove by name
    let isRootAdd = !originalRecord;
    let mainNormalizedRecord = normalizeRouteRecord(record);
    // we might be the child of an alias
    mainNormalizedRecord.aliasOf = originalRecord && originalRecord.record;
    const options = mergeOptions(globalOptions, record);
    // generate an array of records to correctly handle aliases
    const normalizedRecords = [mainNormalizedRecord];
    if ('alias' in record) {
      const aliases =
        typeof record.alias === 'string' ? [record.alias] : record.alias;
      for (const alias of aliases) {
        normalizedRecords.push(
          assign({}, mainNormalizedRecord, {
            // this allows us to hold a copy of the `components` option
            // so that async components cache is hold on the original record
            components: originalRecord
              ? originalRecord.record.components
              : mainNormalizedRecord.components,
            path: alias,
            // we might be the child of an alias
            aliasOf: originalRecord
              ? originalRecord.record
              : mainNormalizedRecord
          })
        );
      }
    }
    let matcher;
    let originalMatcher;
    for (const normalizedRecord of normalizedRecords) {
      let { path } = normalizedRecord;
      // Build up the path for nested routes if the child isn't an absolute
      // route. Only add the / delimiter if the child path isn't empty and if the
      // parent path doesn't have a trailing slash
      if (parent && path[0] !== '/') {
        let parentPath = parent.record.path;
        let connectingSlash =
          parentPath[parentPath.length - 1] === '/' ? '' : '/';
        normalizedRecord.path =
          parent.record.path + (path && connectingSlash + path);
      }
      // create the object before hand so it can be passed to children
      matcher = createRouteRecordMatcher(normalizedRecord, parent, options);
      if (parent && path[0] === '/')
        checkMissingParamsInAbsolutePath(matcher, parent);
      // if we are an alias we must tell the original record that we exist
      // so we can be removed
      if (originalRecord) {
        originalRecord.alias.push(matcher);
        {
          checkSameParams(originalRecord, matcher);
        }
      } else {
        // otherwise, the first record is the original and others are aliases
        originalMatcher = originalMatcher || matcher;
        if (originalMatcher !== matcher) originalMatcher.alias.push(matcher);
        // remove the route if named and only for the top record (avoid in nested calls)
        // this works because the original record is the first one
        if (isRootAdd && record.name && !isAliasRecord(matcher))
          removeRoute(record.name);
      }
      if ('children' in mainNormalizedRecord) {
        let children = mainNormalizedRecord.children;
        for (let i = 0; i < children.length; i++) {
          addRoute(
            children[i],
            matcher,
            originalRecord && originalRecord.children[i]
          );
        }
      }
      // if there was no original record, then the first one was not an alias and all
      // other alias (if any) need to reference this record when adding children
      originalRecord = originalRecord || matcher;
      insertMatcher(matcher);
    }
    return originalMatcher
      ? () => {
          // since other matchers are aliases, they should be removed by the original matcher
          removeRoute(originalMatcher);
        }
      : noop;
  }
  function removeRoute(matcherRef) {
    if (isRouteName(matcherRef)) {
      const matcher = matcherMap.get(matcherRef);
      if (matcher) {
        matcherMap.delete(matcherRef);
        matchers.splice(matchers.indexOf(matcher), 1);
        matcher.children.forEach(removeRoute);
        matcher.alias.forEach(removeRoute);
      }
    } else {
      let index = matchers.indexOf(matcherRef);
      if (index > -1) {
        matchers.splice(index, 1);
        if (matcherRef.record.name) matcherMap.delete(matcherRef.record.name);
        matcherRef.children.forEach(removeRoute);
        matcherRef.alias.forEach(removeRoute);
      }
    }
  }
  function getRoutes() {
    return matchers;
  }
  function insertMatcher(matcher) {
    let i = 0;
    // console.log('i is', { i })
    while (
      i < matchers.length &&
      comparePathParserScore(matcher, matchers[i]) >= 0
    )
      i++;
    // console.log('END i is', { i })
    // while (i < matchers.length && matcher.score <= matchers[i].score) i++
    matchers.splice(i, 0, matcher);
    // only add the original record to the name map
    if (matcher.record.name && !isAliasRecord(matcher))
      matcherMap.set(matcher.record.name, matcher);
  }
  function resolve(location, currentLocation) {
    let matcher;
    let params = {};
    let path;
    let name;
    if ('name' in location && location.name) {
      matcher = matcherMap.get(location.name);
      if (!matcher)
        throw createRouterError(1 /* MATCHER_NOT_FOUND */, {
          location
        });
      name = matcher.record.name;
      params = assign(
        // paramsFromLocation is a new object
        paramsFromLocation(
          currentLocation.params,
          // only keep params that exist in the resolved location
          // TODO: only keep optional params coming from a parent record
          matcher.keys.filter((k) => !k.optional).map((k) => k.name)
        ),
        location.params
      );
      // throws if cannot be stringified
      path = matcher.stringify(params);
    } else if ('path' in location) {
      // no need to resolve the path with the matcher as it was provided
      // this also allows the user to control the encoding
      path = location.path;
      if (path[0] !== '/') {
        warn(
          `The Matcher cannot resolve relative paths but received "${path}". Unless you directly called \`matcher.resolve("${path}")\`, this is probably a bug in vue-router. Please open an issue at https://new-issue.vuejs.org/?repo=vuejs/vue-router-next.`
        );
      }
      matcher = matchers.find((m) => m.re.test(path));
      // matcher should have a value after the loop
      if (matcher) {
        // TODO: dev warning of unused params if provided
        params = matcher.parse(path);
        name = matcher.record.name;
      }
      // location is a relative path
    } else {
      // match by name or path of current route
      matcher = currentLocation.name
        ? matcherMap.get(currentLocation.name)
        : matchers.find((m) => m.re.test(currentLocation.path));
      if (!matcher)
        throw createRouterError(1 /* MATCHER_NOT_FOUND */, {
          location,
          currentLocation
        });
      name = matcher.record.name;
      // since we are navigating to the same location, we don't need to pick the
      // params like when `name` is provided
      params = assign({}, currentLocation.params, location.params);
      path = matcher.stringify(params);
    }
    const matched = [];
    let parentMatcher = matcher;
    while (parentMatcher) {
      // reversed order so parents are at the beginning
      matched.unshift(parentMatcher.record);
      parentMatcher = parentMatcher.parent;
    }
    return {
      name,
      path,
      params,
      matched,
      meta: mergeMetaFields(matched)
    };
  }
  // add initial routes
  routes.forEach((route) => addRoute(route));
  return { addRoute, resolve, removeRoute, getRoutes, getRecordMatcher };
}
function paramsFromLocation(params, keys) {
  let newParams = {};
  for (let key of keys) {
    if (key in params) newParams[key] = params[key];
  }
  return newParams;
}
/**
 * Normalizes a RouteRecordRaw. Creates a copy
 *
 * @param record
 * @returns the normalized version
 */
function normalizeRouteRecord(record) {
  return {
    path: record.path,
    redirect: record.redirect,
    name: record.name,
    meta: record.meta || {},
    aliasOf: undefined,
    beforeEnter: record.beforeEnter,
    props: normalizeRecordProps(record),
    children: record.children || [],
    instances: {},
    leaveGuards: [],
    updateGuards: [],
    enterCallbacks: {},
    components:
      'components' in record
        ? record.components || {}
        : { default: record.component }
  };
}
/**
 * Normalize the optional `props` in a record to always be an object similar to
 * components. Also accept a boolean for components.
 * @param record
 */
function normalizeRecordProps(record) {
  const propsObject = {};
  // props does not exist on redirect records but we can set false directly
  const props = record.props || false;
  if ('component' in record) {
    propsObject.default = props;
  } else {
    // NOTE: we could also allow a function to be applied to every component.
    // Would need user feedback for use cases
    for (let name in record.components)
      propsObject[name] = typeof props === 'boolean' ? props : props[name];
  }
  return propsObject;
}
/**
 * Checks if a record or any of its parent is an alias
 * @param record
 */
function isAliasRecord(record) {
  while (record) {
    if (record.record.aliasOf) return true;
    record = record.parent;
  }
  return false;
}
/**
 * Merge meta fields of an array of records
 *
 * @param matched array of matched records
 */
function mergeMetaFields(matched) {
  return matched.reduce((meta, record) => assign(meta, record.meta), {});
}
function mergeOptions(defaults, partialOptions) {
  let options = {};
  for (let key in defaults) {
    options[key] = key in partialOptions ? partialOptions[key] : defaults[key];
  }
  return options;
}
function isSameParam(a, b) {
  return (
    a.name === b.name &&
    a.optional === b.optional &&
    a.repeatable === b.repeatable
  );
}
function checkSameParams(a, b) {
  for (let key of a.keys) {
    if (!b.keys.find(isSameParam.bind(null, key)))
      return warn(
        `Alias "${b.record.path}" and the original record: "${a.record.path}" should have the exact same param named "${key.name}"`
      );
  }
  for (let key of b.keys) {
    if (!a.keys.find(isSameParam.bind(null, key)))
      return warn(
        `Alias "${b.record.path}" and the original record: "${a.record.path}" should have the exact same param named "${key.name}"`
      );
  }
}
function checkMissingParamsInAbsolutePath(record, parent) {
  for (let key of parent.keys) {
    if (!record.keys.find(isSameParam.bind(null, key)))
      return warn(
        `Absolute path "${record.record.path}" should have the exact same param named "${key.name}" as its parent "${parent.record.path}".`
      );
  }
}

/**
 * Encoding Rules ␣ = Space Path: ␣ " < > # ? { } Query: ␣ " < > # & = Hash: ␣ "
 * < > `
 *
 * On top of that, the RFC3986 (https://tools.ietf.org/html/rfc3986#section-2.2)
 * defines some extra characters to be encoded. Most browsers do not encode them
 * in encodeURI https://github.com/whatwg/url/issues/369, so it may be safer to
 * also encode `!'()*`. Leaving unencoded only ASCII alphanumeric(`a-zA-Z0-9`)
 * plus `-._~`. This extra safety should be applied to query by patching the
 * string returned by encodeURIComponent encodeURI also encodes `[\]^`. `\`
 * should be encoded to avoid ambiguity. Browsers (IE, FF, C) transform a `\`
 * into a `/` if directly typed in. The _backtick_ (`````) should also be
 * encoded everywhere because some browsers like FF encode it when directly
 * written while others don't. Safari and IE don't encode ``"<>{}``` in hash.
 */
// const EXTRA_RESERVED_RE = /[!'()*]/g
// const encodeReservedReplacer = (c: string) => '%' + c.charCodeAt(0).toString(16)
const HASH_RE = /#/g; // %23
const AMPERSAND_RE = /&/g; // %26
const SLASH_RE = /\//g; // %2F
const EQUAL_RE = /=/g; // %3D
const IM_RE = /\?/g; // %3F
const ENC_BRACKET_OPEN_RE = /%5B/g; // [
const ENC_BRACKET_CLOSE_RE = /%5D/g; // ]
const ENC_CARET_RE = /%5E/g; // ^
const ENC_BACKTICK_RE = /%60/g; // `
const ENC_CURLY_OPEN_RE = /%7B/g; // {
const ENC_PIPE_RE = /%7C/g; // |
const ENC_CURLY_CLOSE_RE = /%7D/g; // }
/**
 * Encode characters that need to be encoded on the path, search and hash
 * sections of the URL.
 *
 * @internal
 * @param text - string to encode
 * @returns encoded string
 */
function commonEncode(text) {
  return encodeURI('' + text)
    .replace(ENC_PIPE_RE, '|')
    .replace(ENC_BRACKET_OPEN_RE, '[')
    .replace(ENC_BRACKET_CLOSE_RE, ']');
}
/**
 * Encode characters that need to be encoded on the hash section of the URL.
 *
 * @param text - string to encode
 * @returns encoded string
 */
function encodeHash(text) {
  return commonEncode(text)
    .replace(ENC_CURLY_OPEN_RE, '{')
    .replace(ENC_CURLY_CLOSE_RE, '}')
    .replace(ENC_CARET_RE, '^');
}
/**
 * Encode characters that need to be encoded query keys and values on the query
 * section of the URL.
 *
 * @param text - string to encode
 * @returns encoded string
 */
function encodeQueryProperty(text) {
  return commonEncode(text)
    .replace(HASH_RE, '%23')
    .replace(AMPERSAND_RE, '%26')
    .replace(EQUAL_RE, '%3D')
    .replace(ENC_BACKTICK_RE, '`')
    .replace(ENC_CURLY_OPEN_RE, '{')
    .replace(ENC_CURLY_CLOSE_RE, '}')
    .replace(ENC_CARET_RE, '^');
}
/**
 * Encode characters that need to be encoded on the path section of the URL.
 *
 * @param text - string to encode
 * @returns encoded string
 */
function encodePath(text) {
  return commonEncode(text).replace(HASH_RE, '%23').replace(IM_RE, '%3F');
}
/**
 * Encode characters that need to be encoded on the path section of the URL as a
 * param. This function encodes everything {@link encodePath} does plus the
 * slash (`/`) character.
 *
 * @param text - string to encode
 * @returns encoded string
 */
function encodeParam(text) {
  return encodePath(text).replace(SLASH_RE, '%2F');
}
/**
 * Decode text using `decodeURIComponent`. Returns the original text if it
 * fails.
 *
 * @param text - string to decode
 * @returns decoded string
 */
function decode(text) {
  try {
    return decodeURIComponent('' + text);
  } catch (err) {
    warn(`Error decoding "${text}". Using original value`);
  }
  return '' + text;
}

/**
 * Transforms a queryString into a {@link LocationQuery} object. Accept both, a
 * version with the leading `?` and without Should work as URLSearchParams
 *
 * @param search - search string to parse
 * @returns a query object
 */
function parseQuery(search) {
  const query = {};
  // avoid creating an object with an empty key and empty value
  // because of split('&')
  if (search === '' || search === '?') return query;
  const hasLeadingIM = search[0] === '?';
  const searchParams = (hasLeadingIM ? search.slice(1) : search).split('&');
  for (let i = 0; i < searchParams.length; ++i) {
    let [key, rawValue] = searchParams[i].split('=');
    key = decode(key);
    // avoid decoding null
    let value = rawValue == null ? null : decode(rawValue);
    if (key in query) {
      // an extra variable for ts types
      let currentValue = query[key];
      if (!Array.isArray(currentValue)) {
        currentValue = query[key] = [currentValue];
      }
      currentValue.push(value);
    } else {
      query[key] = value;
    }
  }
  return query;
}
/**
 * Stringifies a {@link LocationQueryRaw} object. Like `URLSearchParams`, it
 * doesn't prepend a `?`
 *
 * @param query - query object to stringify
 * @returns string version of the query without the leading `?`
 */
function stringifyQuery(query) {
  let search = '';
  for (let key in query) {
    if (search.length) search += '&';
    const value = query[key];
    key = encodeQueryProperty(key);
    if (value == null) {
      // only null adds the value
      if (value !== undefined) search += key;
      continue;
    }
    // keep null values
    let values = Array.isArray(value)
      ? value.map((v) => v && encodeQueryProperty(v))
      : [value && encodeQueryProperty(value)];
    for (let i = 0; i < values.length; i++) {
      // only append & with i > 0
      search += (i ? '&' : '') + key;
      if (values[i] != null) search += '=' + values[i];
    }
  }
  return search;
}
/**
 * Transforms a {@link LocationQueryRaw} into a {@link LocationQuery} by casting
 * numbers into strings, removing keys with an undefined value and replacing
 * undefined with null in arrays
 *
 * @param query - query object to normalize
 * @returns a normalized query object
 */
function normalizeQuery(query) {
  const normalizedQuery = {};
  for (let key in query) {
    let value = query[key];
    if (value !== undefined) {
      normalizedQuery[key] = Array.isArray(value)
        ? value.map((v) => (v == null ? null : '' + v))
        : value == null
        ? value
        : '' + value;
    }
  }
  return normalizedQuery;
}

/**
 * Create a list of callbacks that can be reset. Used to create before and after navigation guards list
 */
function useCallbacks() {
  let handlers = [];
  function add(handler) {
    handlers.push(handler);
    return () => {
      const i = handlers.indexOf(handler);
      if (i > -1) handlers.splice(i, 1);
    };
  }
  function reset() {
    handlers = [];
  }
  return {
    add,
    list: () => handlers,
    reset
  };
}

/**
 * Add a navigation guard that triggers whenever the current location is
 * left. Similarly to {@link beforeRouteLeave}, it has access to the
 * component instance as `this`.
 *
 * @param leaveGuard - {@link NavigationGuard}
 */
function onBeforeRouteLeave(leaveGuard) {
  const instance = getCurrentInstance();
  if (!instance) {
    warn$1('onBeforeRouteLeave must be called at the top of a setup function');
    return;
  }
  const activeRecord = inject(matchedRouteKey, {}).value;
  if (!activeRecord) {
    warn$1('onBeforeRouteLeave must be called at the top of a setup function');
    return;
  }
  activeRecord.leaveGuards.push(
    // @ts-ignore do we even want to allow that? Passing the context in a composition api hook doesn't make sense
    leaveGuard.bind(instance.proxy)
  );
}
/**
 * Add a navigation guard that triggers whenever the current location is
 * updated. Similarly to {@link beforeRouteUpdate}, it has access to the
 * component instance as `this`.
 *
 * @param updateGuard - {@link NavigationGuard}
 */
function onBeforeRouteUpdate(updateGuard) {
  const instance = getCurrentInstance();
  if (!instance) {
    warn$1('onBeforeRouteUpdate must be called at the top of a setup function');
    return;
  }
  const activeRecord = inject(matchedRouteKey, {}).value;
  if (!activeRecord) {
    warn$1('onBeforeRouteUpdate must be called at the top of a setup function');
    return;
  }
  activeRecord.updateGuards.push(
    // @ts-ignore do we even want to allow that? Passing the context in a composition api hook doesn't make sense
    updateGuard.bind(instance.proxy)
  );
}
function guardToPromiseFn(guard, to, from, record, name) {
  // keep a reference to the enterCallbackArray to prevent pushing callbacks if a new navigation took place
  const enterCallbackArray =
    record &&
    // name is defined if record is because of the function overload
    (record.enterCallbacks[name] = record.enterCallbacks[name] || []);
  return () =>
    new Promise((resolve, reject) => {
      const next = (valid) => {
        if (valid === false)
          reject(
            createRouterError(4 /* NAVIGATION_ABORTED */, {
              from,
              to
            })
          );
        else if (valid instanceof Error) {
          reject(valid);
        } else if (isRouteLocation(valid)) {
          reject(
            createRouterError(2 /* NAVIGATION_GUARD_REDIRECT */, {
              from: to,
              to: valid
            })
          );
        } else {
          if (
            enterCallbackArray &&
            // since enterCallbackArray is truthy, both record and name also are
            record.enterCallbacks[name] === enterCallbackArray &&
            typeof valid === 'function'
          )
            enterCallbackArray.push(valid);
          resolve();
        }
      };
      // wrapping with Promise.resolve allows it to work with both async and sync guards
      let guardCall = Promise.resolve(
        guard.call(
          record && record.instances[name],
          to,
          from,
          canOnlyBeCalledOnce(next, to, from)
        )
      );
      if (guard.length < 3) guardCall = guardCall.then(next);
      if (guard.length > 2)
        guardCall = guardCall.then(() => {
          // @ts-ignore: _called is added at canOnlyBeCalledOnce
          if (!next._called)
            warn$1(
              `The "next" callback was never called inside of ${
                guard.name ? '"' + guard.name + '"' : ''
              }:\n${guard.toString()}\n. If you are returning a value instead of calling "next", make sure to remove the "next" parameter from your function.`
            );
          return Promise.reject(new Error('Invalid navigation guard'));
        });
      guardCall.catch((err) => reject(err));
    });
}
function canOnlyBeCalledOnce(next, to, from) {
  let called = 0;
  return function () {
    if (called++ === 1)
      warn$1(
        `The "next" callback was called more than once in one navigation guard when going from "${from.fullPath}" to "${to.fullPath}". It should be called exactly one time in each navigation guard. This will fail in production.`
      );
    // @ts-ignore: we put it in the original one because it's easier to check
    next._called = true;
    if (called === 1) next.apply(null, arguments);
  };
}
function extractComponentsGuards(matched, guardType, to, from) {
  const guards = [];
  for (const record of matched) {
    for (const name in record.components) {
      let rawComponent = record.components[name];
      // warn if user wrote import('/component.vue') instead of () => import('./component.vue')
      if ('then' in rawComponent) {
        warn$1(
          `Component "${name}" in record with path "${record.path}" is a Promise instead of a function that returns a Promise. Did you write "import('./MyPage.vue')" instead of "() => import('./MyPage.vue')"? This will break in production if not fixed.`
        );
        let promise = rawComponent;
        rawComponent = () => promise;
      }
      // skip update and leave guards if the route component is not mounted
      if (guardType !== 'beforeRouteEnter' && !record.instances[name]) continue;
      if (isRouteComponent(rawComponent)) {
        // __vccOpts is added by vue-class-component and contain the regular options
        let options = rawComponent.__vccOpts || rawComponent;
        const guard = options[guardType];
        guard && guards.push(guardToPromiseFn(guard, to, from, record, name));
      } else {
        // start requesting the chunk already
        let componentPromise = rawComponent();
        if (!('catch' in componentPromise)) {
          warn$1(
            `Component "${name}" in record with path "${record.path}" is a function that does not return a Promise. If you were passing a functional component, make sure to add a "displayName" to the component. This will break in production if not fixed.`
          );
          componentPromise = Promise.resolve(componentPromise);
        } else {
          componentPromise = componentPromise.catch(() => null);
        }
        guards.push(() =>
          componentPromise.then((resolved) => {
            if (!resolved)
              return Promise.reject(
                new Error(
                  `Couldn't resolve component "${name}" for the following record with path "${record.path}"`
                )
              );
            const resolvedComponent = isESModule(resolved)
              ? resolved.default
              : resolved;
            // replace the function with the resolved component
            record.components[name] = resolvedComponent;
            // @ts-ignore: the options types are not propagated to Component
            const guard = resolvedComponent[guardType];
            return guard && guardToPromiseFn(guard, to, from, record, name)();
          })
        );
      }
    }
  }
  return guards;
}
/**
 * Allows differentiating lazy components from functional components and vue-class-component
 * @param component
 */
function isRouteComponent(component) {
  return (
    typeof component === 'object' ||
    'displayName' in component ||
    'props' in component ||
    '__vccOpts' in component
  );
}

// TODO: we could allow currentRoute as a prop to expose `isActive` and
// `isExactActive` behavior should go through an RFC
function useLink(props) {
  const router = inject(routerKey);
  const currentRoute = inject(routeLocationKey);
  const route = computed(() => router.resolve(unref(props.to)));
  const activeRecordIndex = computed(() => {
    let { matched } = route.value;
    let { length } = matched;
    const routeMatched = matched[length - 1];
    let currentMatched = currentRoute.matched;
    if (!routeMatched || !currentMatched.length) return -1;
    let index = currentMatched.findIndex(
      isSameRouteRecord.bind(null, routeMatched)
    );
    if (index > -1) return index;
    // possible parent record
    let parentRecordPath = getOriginalPath(matched[length - 2]);
    return (
      // we are dealing with nested routes
      length > 1 &&
        // if the have the same path, this link is referring to the empty child
        // are we currently are on a different child of the same parent
        getOriginalPath(routeMatched) === parentRecordPath &&
        // avoid comparing the child with its parent
        currentMatched[currentMatched.length - 1].path !== parentRecordPath
        ? currentMatched.findIndex(
            isSameRouteRecord.bind(null, matched[length - 2])
          )
        : index
    );
  });
  const isActive = computed(
    () =>
      activeRecordIndex.value > -1 &&
      includesParams(currentRoute.params, route.value.params)
  );
  const isExactActive = computed(
    () =>
      activeRecordIndex.value > -1 &&
      activeRecordIndex.value === currentRoute.matched.length - 1 &&
      isSameRouteLocationParams(currentRoute.params, route.value.params)
  );
  function navigate(e = {}) {
    if (guardEvent(e))
      return router[unref(props.replace) ? 'replace' : 'push'](unref(props.to));
    return Promise.resolve();
  }
  return {
    route,
    href: computed(() => route.value.href),
    isActive,
    isExactActive,
    navigate
  };
}
const RouterLinkImpl = defineComponent({
  name: 'RouterLink',
  props: {
    to: {
      type: [String, Object],
      required: true
    },
    activeClass: String,
    // inactiveClass: String,
    exactActiveClass: String,
    custom: Boolean,
    ariaCurrentValue: {
      type: String,
      default: 'page'
    }
  },
  setup(props, { slots, attrs }) {
    const link = reactive(useLink(props));
    const { options } = inject(routerKey);
    const elClass = computed(() => ({
      [getLinkClass(
        props.activeClass,
        options.linkActiveClass,
        'router-link-active'
      )]: link.isActive,
      // [getLinkClass(
      //   props.inactiveClass,
      //   options.linkInactiveClass,
      //   'router-link-inactive'
      // )]: !link.isExactActive,
      [getLinkClass(
        props.exactActiveClass,
        options.linkExactActiveClass,
        'router-link-exact-active'
      )]: link.isExactActive
    }));
    return () => {
      const children = slots.default && slots.default(link);
      return props.custom
        ? children
        : h(
            'a',
            assign(
              {
                'aria-current': link.isExactActive
                  ? props.ariaCurrentValue
                  : null,
                onClick: link.navigate,
                href: link.href
              },
              attrs,
              {
                class: elClass.value
              }
            ),
            children
          );
    };
  }
});
// export the public type for h/tsx inference
// also to avoid inline import() in generated d.ts files
const RouterLink = RouterLinkImpl;
function guardEvent(e) {
  // don't redirect with control keys
  if (e.metaKey || e.altKey || e.ctrlKey || e.shiftKey) return;
  // don't redirect when preventDefault called
  if (e.defaultPrevented) return;
  // don't redirect on right click
  if (e.button !== undefined && e.button !== 0) return;
  // don't redirect if `target="_blank"`
  // @ts-ignore getAttribute does exist
  if (e.currentTarget && e.currentTarget.getAttribute) {
    // @ts-ignore getAttribute exists
    const target = e.currentTarget.getAttribute('target');
    if (/\b_blank\b/i.test(target)) return;
  }
  // this may be a Weex event which doesn't have this method
  if (e.preventDefault) e.preventDefault();
  return true;
}
function includesParams(outer, inner) {
  for (let key in inner) {
    let innerValue = inner[key];
    let outerValue = outer[key];
    if (typeof innerValue === 'string') {
      if (innerValue !== outerValue) return false;
    } else {
      if (
        !Array.isArray(outerValue) ||
        outerValue.length !== innerValue.length ||
        innerValue.some((value, i) => value !== outerValue[i])
      )
        return false;
    }
  }
  return true;
}
/**
 * Get the original path value of a record by following its aliasOf
 * @param record
 */
function getOriginalPath(record) {
  return record ? (record.aliasOf ? record.aliasOf.path : record.path) : '';
}
/**
 * Utility class to get the active class based on defaults.
 * @param propClass
 * @param globalClass
 * @param defaultClass
 */
let getLinkClass = (propClass, globalClass, defaultClass) =>
  propClass != null
    ? propClass
    : globalClass != null
    ? globalClass
    : defaultClass;

const RouterViewImpl = defineComponent({
  name: 'RouterView',
  props: {
    name: {
      type: String,
      default: 'default'
    },
    route: Object
  },
  setup(props, { attrs, slots }) {
    warnDeprecatedUsage();
    const injectedRoute = inject(routeLocationKey);
    const depth = inject(viewDepthKey, 0);
    const matchedRouteRef = computed(
      () => (props.route || injectedRoute).matched[depth]
    );
    provide(viewDepthKey, depth + 1);
    provide(matchedRouteKey, matchedRouteRef);
    const viewRef = ref();
    return () => {
      const route = props.route || injectedRoute;
      const matchedRoute = matchedRouteRef.value;
      const ViewComponent = matchedRoute && matchedRoute.components[props.name];
      if (!ViewComponent) {
        return slots.default
          ? slots.default({ Component: ViewComponent, route })
          : null;
      }
      // props from route configration
      const routePropsOption = matchedRoute.props[props.name];
      const routeProps = routePropsOption
        ? routePropsOption === true
          ? route.params
          : typeof routePropsOption === 'function'
          ? routePropsOption(route)
          : routePropsOption
        : null;
      // we need the value at the time we render because when we unmount, we
      // navigated to a different location so the value is different
      const currentName = props.name;
      const onVnodeMounted = () => {
        matchedRoute.instances[currentName] = viewRef.value;
        (matchedRoute.enterCallbacks[currentName] || []).forEach((callback) =>
          callback(viewRef.value)
        );
      };
      const onVnodeUnmounted = () => {
        // remove the instance reference to prevent leak
        matchedRoute.instances[currentName] = null;
      };
      const component = h(
        ViewComponent,
        assign({}, routeProps, attrs, {
          onVnodeMounted,
          onVnodeUnmounted,
          ref: viewRef
        })
      );
      return (
        // pass the vnode to the slot as a prop.
        // h and <component :is="..."> both accept vnodes
        slots.default
          ? slots.default({ Component: component, route })
          : component
      );
    };
  }
});
// export the public type for h/tsx inference
// also to avoid inline import() in generated d.ts files
const RouterView = RouterViewImpl;
// warn against deprecated usage with <transition> & <keep-alive>
// due to functional component being no longer eager in Vue 3
function warnDeprecatedUsage() {
  const instance = getCurrentInstance();
  const parentName = instance.parent && instance.parent.type.name;
  if (
    parentName &&
    (parentName === 'KeepAlive' || parentName.includes('Transition'))
  ) {
    const comp = parentName === 'KeepAlive' ? 'keep-alive' : 'transition';
    warn(
      `<router-view> can no longer be used directly inside <transition> or <keep-alive>.\n` +
        `Use slot props instead:\n\n` +
        `<router-view v-slot="{ Component }">\n` +
        `  <${comp}>\n` +
        `    <component :is="Component" />\n` +
        `  </${comp}>\n` +
        `</router-view>`
    );
  }
}

/**
 * Create a Router instance that can be used on a Vue app.
 *
 * @param options - {@link RouterOptions}
 */
function createRouter(options) {
  const matcher = createRouterMatcher(options.routes, options);
  let parseQuery$1 = options.parseQuery || parseQuery;
  let stringifyQuery$1 = options.stringifyQuery || stringifyQuery;
  let { scrollBehavior } = options;
  let routerHistory = options.history;
  const beforeGuards = useCallbacks();
  const beforeResolveGuards = useCallbacks();
  const afterGuards = useCallbacks();
  const currentRoute = shallowRef(START_LOCATION_NORMALIZED);
  let pendingLocation = START_LOCATION_NORMALIZED;
  // leave the scrollRestoration if no scrollBehavior is provided
  if (isBrowser && scrollBehavior && 'scrollRestoration' in history) {
    history.scrollRestoration = 'manual';
  }
  const normalizeParams = applyToParams.bind(
    null,
    (paramValue) => '' + paramValue
  );
  const encodeParams = applyToParams.bind(null, encodeParam);
  const decodeParams = applyToParams.bind(null, decode);
  function addRoute(parentOrRoute, route) {
    let parent;
    let record;
    if (isRouteName(parentOrRoute)) {
      parent = matcher.getRecordMatcher(parentOrRoute);
      record = route;
    } else {
      record = parentOrRoute;
    }
    return matcher.addRoute(record, parent);
  }
  function removeRoute(name) {
    let recordMatcher = matcher.getRecordMatcher(name);
    if (recordMatcher) {
      matcher.removeRoute(recordMatcher);
    } else {
      warn(`Cannot remove non-existent route "${String(name)}"`);
    }
  }
  function getRoutes() {
    return matcher.getRoutes().map((routeMatcher) => routeMatcher.record);
  }
  function hasRoute(name) {
    return !!matcher.getRecordMatcher(name);
  }
  function resolve(rawLocation, currentLocation) {
    // const objectLocation = routerLocationAsObject(rawLocation)
    // we create a copy to modify it later
    currentLocation = assign({}, currentLocation || currentRoute.value);
    if (typeof rawLocation === 'string') {
      let locationNormalized = parseURL(
        parseQuery$1,
        rawLocation,
        currentLocation.path
      );
      let matchedRoute = matcher.resolve(
        { path: locationNormalized.path },
        currentLocation
      );
      let href = routerHistory.createHref(locationNormalized.fullPath);
      {
        if (href.startsWith('//'))
          warn(
            `Location "${rawLocation}" resolved to "${href}". A resolved location cannot start with multiple slashes.`
          );
        else if (!matchedRoute.matched.length) {
          warn(`No match found for location with path "${rawLocation}"`);
        }
      }
      // locationNormalized is always a new object
      return assign(locationNormalized, matchedRoute, {
        params: decodeParams(matchedRoute.params),
        redirectedFrom: undefined,
        href
      });
    }
    let matcherLocation;
    // path could be relative in object as well
    if ('path' in rawLocation) {
      if (
        'params' in rawLocation &&
        !('name' in rawLocation) &&
        Object.keys(rawLocation.params).length
      ) {
        warn(
          `Path "${rawLocation.path}" was passed with params but they will be ignored. Use a named route alongside params instead.`
        );
      }
      matcherLocation = assign({}, rawLocation, {
        path: parseURL(parseQuery$1, rawLocation.path, currentLocation.path)
          .path
      });
    } else {
      // pass encoded values to the matcher so it can produce encoded path and fullPath
      matcherLocation = assign({}, rawLocation, {
        params: encodeParams(rawLocation.params)
      });
      // current location params are decoded, we need to encode them in case the
      // matcher merges the params
      currentLocation.params = encodeParams(currentLocation.params);
    }
    let matchedRoute = matcher.resolve(matcherLocation, currentLocation);
    const hash = encodeHash(rawLocation.hash || '');
    if (hash && !hash.startsWith('#')) {
      warn(
        `A \`hash\` should always start with the character "#". Replace "${hash}" with "#${hash}".`
      );
    }
    // decoding them) the matcher might have merged current location params so
    // we need to run the decoding again
    matchedRoute.params = normalizeParams(decodeParams(matchedRoute.params));
    const fullPath = stringifyURL(
      stringifyQuery$1,
      assign({}, rawLocation, {
        hash,
        path: matchedRoute.path
      })
    );
    let href = routerHistory.createHref(fullPath);
    {
      if (href.startsWith('//'))
        warn(
          `Location "${rawLocation}" resolved to "${href}". A resolved location cannot start with multiple slashes.`
        );
      else if (!matchedRoute.matched.length) {
        warn(
          `No match found for location with path "${
            'path' in rawLocation ? rawLocation.path : rawLocation
          }"`
        );
      }
    }
    return assign(
      {
        fullPath,
        // keep the hash encoded so fullPath is effectively path + encodedQuery +
        // hash
        hash,
        query:
          // if the user is using a custom query lib like qs, we might have
          // nested objects, so we keep the query as is, meaning it can contain
          // numbers at `$route.query`, but at the point, the user will have to
          // use their own type anyway.
          // https://github.com/vuejs/vue-router-next/issues/328#issuecomment-649481567
          stringifyQuery$1 === stringifyQuery
            ? normalizeQuery(rawLocation.query)
            : rawLocation.query
      },
      matchedRoute,
      {
        redirectedFrom: undefined,
        href
      }
    );
  }
  function locationAsObject(to) {
    return typeof to === 'string' ? { path: to } : assign({}, to);
  }
  function checkCanceledNavigation(to, from) {
    if (pendingLocation !== to) {
      return createRouterError(8 /* NAVIGATION_CANCELLED */, {
        from,
        to
      });
    }
  }
  function push(to) {
    return pushWithRedirect(to);
  }
  function replace(to) {
    return push(assign(locationAsObject(to), { replace: true }));
  }
  function pushWithRedirect(to, redirectedFrom) {
    const targetLocation = (pendingLocation = resolve(to));
    const from = currentRoute.value;
    const data = to.state;
    const force = to.force;
    // to could be a string where `replace` is a function
    const replace = to.replace === true;
    const lastMatched =
      targetLocation.matched[targetLocation.matched.length - 1];
    if (lastMatched && lastMatched.redirect) {
      const { redirect } = lastMatched;
      // transform it into an object to pass the original RouteLocaleOptions
      let newTargetLocation = locationAsObject(
        typeof redirect === 'function' ? redirect(targetLocation) : redirect
      );
      if (!('path' in newTargetLocation) && !('name' in newTargetLocation)) {
        warn(
          `Invalid redirect found:\n${JSON.stringify(
            newTargetLocation,
            null,
            2
          )}\n when navigating to "${
            targetLocation.fullPath
          }". A redirect must contain a name or path. This will break in production.`
        );
        return Promise.reject(new Error('Invalid redirect'));
      }
      return pushWithRedirect(
        assign(
          {
            query: targetLocation.query,
            hash: targetLocation.hash,
            params: targetLocation.params
          },
          newTargetLocation,
          {
            state: data,
            force,
            replace
          }
        ),
        // keep original redirectedFrom if it exists
        redirectedFrom || targetLocation
      );
    }
    // if it was a redirect we already called `pushWithRedirect` above
    const toLocation = targetLocation;
    toLocation.redirectedFrom = redirectedFrom;
    let failure;
    if (!force && isSameRouteLocation(stringifyQuery$1, from, targetLocation)) {
      failure = createRouterError(16 /* NAVIGATION_DUPLICATED */, {
        to: toLocation,
        from
      });
      // trigger scroll to allow scrolling to the same anchor
      handleScroll(
        from,
        from,
        // this is a push, the only way for it to be triggered from a
        // history.listen is with a redirect, which makes it become a pus
        true,
        // This cannot be the first navigation because the initial location
        // cannot be manually navigated to
        false
      );
    }
    return (failure ? Promise.resolve(failure) : navigate(toLocation, from))
      .catch((error) => {
        if (
          isNavigationFailure(
            error,
            4 /* NAVIGATION_ABORTED */ |
            8 /* NAVIGATION_CANCELLED */ |
              2 /* NAVIGATION_GUARD_REDIRECT */
          )
        ) {
          return error;
        }
        // unknown error, rejects
        return triggerError(error);
      })
      .then((failure) => {
        if (failure) {
          if (isNavigationFailure(failure, 2 /* NAVIGATION_GUARD_REDIRECT */)) {
            if (
              // we are redirecting to the same location we were already at
              isSameRouteLocation(
                stringifyQuery$1,
                resolve(failure.to),
                toLocation
              ) &&
              // and we have done it a couple of times
              redirectedFrom &&
              // @ts-ignore
              (redirectedFrom._count = redirectedFrom._count
                ? // @ts-ignore
                  redirectedFrom._count + 1
                : 1) > 10
            ) {
              warn(
                `Detected an infinite redirection in a navigation guard when going from "${from.fullPath}" to "${toLocation.fullPath}". Aborting to avoid a Stack Overflow. This will break in production if not fixed.`
              );
              return Promise.reject(
                new Error('Infinite redirect in navigation guard')
              );
            }
            return pushWithRedirect(
              // keep options
              assign(locationAsObject(failure.to), {
                state: data,
                force,
                replace
              }),
              // preserve the original redirectedFrom if any
              redirectedFrom || toLocation
            );
          }
        } else {
          // if we fail we don't finalize the navigation
          failure = finalizeNavigation(toLocation, from, true, replace, data);
        }
        triggerAfterEach(toLocation, from, failure);
        return failure;
      });
  }
  /**
   * Helper to reject and skip all navigation guards if a new navigation happened
   * @param to
   * @param from
   */
  function checkCanceledNavigationAndReject(to, from) {
    const error = checkCanceledNavigation(to, from);
    return error ? Promise.reject(error) : Promise.resolve();
  }
  // TODO: refactor the whole before guards by internally using router.beforeEach
  function navigate(to, from) {
    let guards;
    const [
      leavingRecords,
      updatingRecords,
      enteringRecords
    ] = extractChangingRecords(to, from);
    // all components here have been resolved once because we are leaving
    guards = extractComponentsGuards(
      leavingRecords.reverse(),
      'beforeRouteLeave',
      to,
      from
    );
    // leavingRecords is already reversed
    for (const record of leavingRecords) {
      for (const guard of record.leaveGuards) {
        guards.push(guardToPromiseFn(guard, to, from));
      }
    }
    const canceledNavigationCheck = checkCanceledNavigationAndReject.bind(
      null,
      to,
      from
    );
    guards.push(canceledNavigationCheck);
    // run the queue of per route beforeRouteLeave guards
    return (
      runGuardQueue(guards)
        .then(() => {
          // check global guards beforeEach
          guards = [];
          for (const guard of beforeGuards.list()) {
            guards.push(guardToPromiseFn(guard, to, from));
          }
          guards.push(canceledNavigationCheck);
          return runGuardQueue(guards);
        })
        .then(() => {
          // check in components beforeRouteUpdate
          guards = extractComponentsGuards(
            updatingRecords,
            'beforeRouteUpdate',
            to,
            from
          );
          for (const record of updatingRecords) {
            for (const guard of record.updateGuards) {
              guards.push(guardToPromiseFn(guard, to, from));
            }
          }
          guards.push(canceledNavigationCheck);
          // run the queue of per route beforeEnter guards
          return runGuardQueue(guards);
        })
        .then(() => {
          // check the route beforeEnter
          guards = [];
          for (const record of to.matched) {
            // do not trigger beforeEnter on reused views
            if (record.beforeEnter && from.matched.indexOf(record) < 0) {
              if (Array.isArray(record.beforeEnter)) {
                for (const beforeEnter of record.beforeEnter)
                  guards.push(guardToPromiseFn(beforeEnter, to, from));
              } else {
                guards.push(guardToPromiseFn(record.beforeEnter, to, from));
              }
            }
          }
          guards.push(canceledNavigationCheck);
          // run the queue of per route beforeEnter guards
          return runGuardQueue(guards);
        })
        .then(() => {
          // NOTE: at this point to.matched is normalized and does not contain any () => Promise<Component>
          // clear existing enterCallbacks, these are added by extractComponentsGuards
          to.matched.forEach((record) => (record.enterCallbacks = {}));
          // check in-component beforeRouteEnter
          guards = extractComponentsGuards(
            enteringRecords,
            'beforeRouteEnter',
            to,
            from
          );
          guards.push(canceledNavigationCheck);
          // run the queue of per route beforeEnter guards
          return runGuardQueue(guards);
        })
        .then(() => {
          // check global guards beforeResolve
          guards = [];
          for (const guard of beforeResolveGuards.list()) {
            guards.push(guardToPromiseFn(guard, to, from));
          }
          guards.push(canceledNavigationCheck);
          return runGuardQueue(guards);
        })
        // catch any navigation canceled
        .catch((err) =>
          isNavigationFailure(err, 8 /* NAVIGATION_CANCELLED */)
            ? err
            : Promise.reject(err)
        )
    );
  }
  function triggerAfterEach(to, from, failure) {
    // navigation is confirmed, call afterGuards
    // TODO: wrap with error handlers
    for (const guard of afterGuards.list()) guard(to, from, failure);
  }
  /**
   * - Cleans up any navigation guards
   * - Changes the url if necessary
   * - Calls the scrollBehavior
   */
  function finalizeNavigation(toLocation, from, isPush, replace, data) {
    // a more recent navigation took place
    const error = checkCanceledNavigation(toLocation, from);
    if (error) return error;
    const [leavingRecords] = extractChangingRecords(toLocation, from);
    for (const record of leavingRecords) {
      // remove registered guards from removed matched records
      record.leaveGuards = [];
      record.updateGuards = [];
      // free the references
      record.instances = {};
      record.enterCallbacks = {};
    }
    // only consider as push if it's not the first navigation
    const isFirstNavigation = from === START_LOCATION_NORMALIZED;
    const state = !isBrowser ? {} : history.state;
    // change URL only if the user did a push/replace and if it's not the initial navigation because
    // it's just reflecting the url
    if (isPush) {
      // on the initial navigation, we want to reuse the scroll position from
      // history state if it exists
      if (replace || isFirstNavigation)
        routerHistory.replace(
          toLocation.fullPath,
          assign(
            {
              scroll: isFirstNavigation && state && state.scroll
            },
            data
          )
        );
      else routerHistory.push(toLocation.fullPath, data);
    }
    // accept current navigation
    currentRoute.value = toLocation;
    handleScroll(toLocation, from, isPush, isFirstNavigation);
    markAsReady();
  }
  let removeHistoryListener;
  // attach listener to history to trigger navigations
  function setupListeners() {
    removeHistoryListener = routerHistory.listen((to, _from, info) => {
      // cannot be a redirect route because it was in history
      const toLocation = resolve(to);
      pendingLocation = toLocation;
      const from = currentRoute.value;
      // TODO: should be moved to web history?
      if (isBrowser) {
        saveScrollPosition(
          getScrollKey(from.fullPath, info.delta),
          computeScrollPosition()
        );
      }
      navigate(toLocation, from)
        .catch((error) => {
          if (
            isNavigationFailure(
              error,
              4 /* NAVIGATION_ABORTED */ | 8 /* NAVIGATION_CANCELLED */
            )
          ) {
            return error;
          }
          if (isNavigationFailure(error, 2 /* NAVIGATION_GUARD_REDIRECT */)) {
            // do not restore history on unknown direction
            if (info.delta) routerHistory.go(-info.delta, false);
            // the error is already handled by router.push we just want to avoid
            // logging the error
            pushWithRedirect(
              error.to,
              toLocation
              // avoid an uncaught rejection
            ).catch(noop);
            // avoid the then branch
            return Promise.reject();
          }
          // do not restore history on unknown direction
          if (info.delta) routerHistory.go(-info.delta, false);
          // unrecognized error, transfer to the global handler
          return triggerError(error);
        })
        .then((failure) => {
          failure =
            failure ||
            finalizeNavigation(
              // after navigation, all matched components are resolved
              toLocation,
              from,
              false
            );
          // revert the navigation
          if (failure && info.delta) routerHistory.go(-info.delta, false);
          triggerAfterEach(toLocation, from, failure);
        })
        .catch(noop);
    });
  }
  // Initialization and Errors
  let readyHandlers = useCallbacks();
  let errorHandlers = useCallbacks();
  let ready;
  /**
   * Trigger errorHandlers added via onError and throws the error as well
   * @param error - error to throw
   * @returns the error as a rejected promise
   */
  function triggerError(error) {
    markAsReady(error);
    errorHandlers.list().forEach((handler) => handler(error));
    return Promise.reject(error);
  }
  /**
   * Returns a Promise that resolves or reject when the router has finished its
   * initial navigation. This will be automatic on client but requires an
   * explicit `router.push` call on the server. This behavior can change
   * depending on the history implementation used e.g. the defaults history
   * implementation (client only) triggers this automatically but the memory one
   * (should be used on server) doesn't
   */
  function isReady() {
    if (ready && currentRoute.value !== START_LOCATION_NORMALIZED)
      return Promise.resolve();
    return new Promise((resolve, reject) => {
      readyHandlers.add([resolve, reject]);
    });
  }
  /**
   * Mark the router as ready, resolving the promised returned by isReady(). Can
   * only be called once, otherwise does nothing.
   * @param err - optional error
   */
  function markAsReady(err) {
    if (ready) return;
    ready = true;
    setupListeners();
    readyHandlers
      .list()
      .forEach(([resolve, reject]) => (err ? reject(err) : resolve()));
    readyHandlers.reset();
  }
  // Scroll behavior
  function handleScroll(to, from, isPush, isFirstNavigation) {
    if (!isBrowser || !scrollBehavior) return Promise.resolve();
    let scrollPosition =
      (!isPush && getSavedScrollPosition(getScrollKey(to.fullPath, 0))) ||
      ((isFirstNavigation || !isPush) &&
        history.state &&
        history.state.scroll) ||
      null;
    return nextTick()
      .then(() => scrollBehavior(to, from, scrollPosition))
      .then((position) => position && scrollToPosition(position))
      .catch(triggerError);
  }
  function go(delta) {
    return new Promise((resolve, reject) => {
      let removeError = errorHandlers.add((err) => {
        removeError();
        removeAfterEach();
        reject(err);
      });
      let removeAfterEach = afterGuards.add((_to, _from, failure) => {
        removeError();
        removeAfterEach();
        resolve(failure);
      });
      routerHistory.go(delta);
    });
  }
  let started;
  const installedApps = new Set();
  const router = {
    currentRoute,
    addRoute,
    removeRoute,
    hasRoute,
    getRoutes,
    resolve,
    options,
    push,
    replace,
    go,
    back: () => go(-1),
    forward: () => go(1),
    beforeEach: beforeGuards.add,
    beforeResolve: beforeResolveGuards.add,
    afterEach: afterGuards.add,
    onError: errorHandlers.add,
    isReady,
    install(app) {
      const router = this;
      app.component('RouterLink', RouterLink);
      app.component('RouterView', RouterView);
      app.config.globalProperties.$router = router;
      Object.defineProperty(app.config.globalProperties, '$route', {
        get: () => unref(currentRoute)
      });
      // this initial navigation is only necessary on client, on server it doesn't
      // make sense because it will create an extra unnecessary navigation and could
      // lead to problems
      if (
        isBrowser &&
        // used for the initial navigation client side to avoid pushing
        // multiple times when the router is used in multiple apps
        !started &&
        currentRoute.value === START_LOCATION_NORMALIZED
      ) {
        // see above
        started = true;
        push(routerHistory.location).catch((err) => {
          warn('Unexpected error when starting the router:', err);
        });
      }
      const reactiveRoute = {};
      for (let key in START_LOCATION_NORMALIZED) {
        // @ts-ignore: the key matches
        reactiveRoute[key] = computed(() => currentRoute.value[key]);
      }
      app.provide(routerKey, router);
      app.provide(routeLocationKey, reactive(reactiveRoute));
      let unmountApp = app.unmount;
      installedApps.add(app);
      app.unmount = function () {
        installedApps.delete(app);
        if (installedApps.size < 1) {
          removeHistoryListener();
          currentRoute.value = START_LOCATION_NORMALIZED;
          started = false;
          ready = false;
        }
        unmountApp.call(this, arguments);
      };
    }
  };
  return router;
}
function runGuardQueue(guards) {
  return guards.reduce(
    (promise, guard) => promise.then(() => guard()),
    Promise.resolve()
  );
}
function extractChangingRecords(to, from) {
  const leavingRecords = [];
  const updatingRecords = [];
  const enteringRecords = [];
  const len = Math.max(from.matched.length, to.matched.length);
  for (let i = 0; i < len; i++) {
    const recordFrom = from.matched[i];
    if (recordFrom) {
      if (to.matched.indexOf(recordFrom) < 0) leavingRecords.push(recordFrom);
      else updatingRecords.push(recordFrom);
    }
    const recordTo = to.matched[i];
    if (recordTo) {
      // the type doesn't matter because we are comparing per reference
      if (from.matched.indexOf(recordTo) < 0) enteringRecords.push(recordTo);
    }
  }
  return [leavingRecords, updatingRecords, enteringRecords];
}

function useRouter() {
  return inject(routerKey);
}
function useRoute() {
  return inject(routeLocationKey);
}

export {
  NavigationFailureType,
  RouterLink,
  RouterView,
  START_LOCATION_NORMALIZED as START_LOCATION,
  createMemoryHistory,
  createRouter,
  createRouterMatcher,
  createWebHashHistory,
  createWebHistory,
  isNavigationFailure,
  onBeforeRouteLeave,
  onBeforeRouteUpdate,
  parseQuery,
  stringifyQuery,
  useLink,
  useRoute,
  useRouter
};
