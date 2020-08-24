import { App } from 'vue';
import { Component } from 'vue';
import { ComponentPublicInstance } from 'vue';
import { ComputedRef } from 'vue';
import { Ref } from 'vue';
import { VNodeProps } from 'vue';

declare type Awaitable<T> = T | Promise<T>;

/**
 * Creates a in-memory based history. The main purpose of this history is to handle SSR. It starts in a special location that is nowhere.
 * It's up to the user to replace that location with the starter location.
 * @param base - Base applied to all urls, defaults to '/'
 * @returns a history object that can be passed to the router constructor
 */
export declare function createMemoryHistory(base?: string): RouterHistory;

/**
 * Create a Router instance that can be used on a Vue app.
 *
 * @param options - {@link RouterOptions}
 */
export declare function createRouter(options: RouterOptions): Router;

/**
 * Creates a Router Matcher.
 *
 * @internal
 * @param routes - array of initial routes
 * @param globalOptions - global route options
 */
export declare function createRouterMatcher(routes: RouteRecordRaw[], globalOptions: PathParserOptions): RouterMatcher;

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
export declare function createWebHashHistory(base?: string): RouterHistory;

export declare function createWebHistory(base?: string): RouterHistory;

/**
 * Internal type to define an ErrorHandler
 * @internal
 */
export declare type ErrorHandler = (error: any) => any;

/**
 * Flags so we can combine them when checking for multiple errors
 */
declare const enum ErrorTypes {
    MATCHER_NOT_FOUND = 1,
    NAVIGATION_GUARD_REDIRECT = 2,
    NAVIGATION_ABORTED = 4,
    NAVIGATION_CANCELLED = 8,
    NAVIGATION_DUPLICATED = 16
}

declare type HistoryLocation = string;

declare interface HistoryState {
    [x: number]: HistoryStateValue;
    [x: string]: HistoryStateValue;
}

declare interface HistoryStateArray extends Array<HistoryStateValue> {
}

declare type HistoryStateValue = string | number | boolean | null | undefined | HistoryState | HistoryStateArray;

/**
 * Check if an object is a {@link NavigationFailure}.
 *
 * @example
 * ```js
 * import { isNavigationFailure, NavigationFailureType } from 'vue-router'
 *
 * router.afterEach((to, from, failure) => {
 *   // Any kind of navigation failure
 *   if (isNavigationFailure(failure)) {
 *     // ...
 *   }
 *   // Only duplicated navigations
 *   if (isNavigationFailure(failure, NavigationFailureType.duplicated)) {
 *     // ...
 *   }
 *   // Aborted or canceled navigations
 *   if (isNavigationFailure(failure, NavigationFailureType.aborted | NavigationFailureType.canceled)) {
 *     // ...
 *   }
 * })
 * ```
 * @param error - possible {@link NavigationFailure}
 * @param type - optional types to check for
 */
export declare function isNavigationFailure(error: any, type?: ErrorTypes.NAVIGATION_GUARD_REDIRECT): error is NavigationRedirectError;

export declare function isNavigationFailure(error: any, type?: ErrorTypes | NavigationFailureType): error is NavigationFailure;

declare type Lazy<T> = () => Promise<T>;

declare interface LocationAsName {
    name: RouteRecordName;
    params?: RouteParams;
}

declare interface LocationAsNameRaw {
    name: RouteRecordName;
    params?: RouteParamsRaw;
}

declare interface LocationAsPath {
    path: string;
}

declare interface LocationAsRelative {
    params?: RouteParams;
}

declare interface LocationAsRelativeRaw {
    params?: RouteParamsRaw;
}

/**
 * Normalized query object that appears in {@link RouteLocationNormalized}
 *
 * @public
 */
export declare type LocationQuery = Record<string, LocationQueryValue | LocationQueryValue[]>;

/**
 * Loose {@link LocationQuery} object that can be passed to functions like
 * {@link Router.push} and {@link Router.replace} or anywhere when creating a
 * {@link RouteLocationRaw}
 *
 * @public
 */
export declare type LocationQueryRaw = Record<string | number, LocationQueryValueRaw | LocationQueryValueRaw[]>;

/**
 * Possible values in normalized {@link LocationQuery}
 *
 * @internal
 */
export declare type LocationQueryValue = string | null;

/**
 * Possible values when defining a query
 *
 * @internal
 */
declare type LocationQueryValueRaw = LocationQueryValue | number | undefined;

declare interface MatcherLocation extends Pick<RouteLocation, 'name' | 'path' | 'params' | 'matched' | 'meta'> {
}

declare type MatcherLocationRaw = LocationAsPath | LocationAsName | LocationAsRelative;

declare interface NavigationCallback {
    (to: HistoryLocation, from: HistoryLocation, information: NavigationInformation): void;
}

declare enum NavigationDirection {
    back = "back",
    forward = "forward",
    unknown = ""
}

export declare interface NavigationFailure extends RouterErrorBase {
    type: ErrorTypes.NAVIGATION_CANCELLED | ErrorTypes.NAVIGATION_ABORTED | ErrorTypes.NAVIGATION_DUPLICATED;
    from: RouteLocationNormalized;
    to: RouteLocationNormalized;
}

export declare enum NavigationFailureType {
    aborted = 4,
    cancelled = 8,
    duplicated = 16
}

export declare interface NavigationGuard {
    (to: RouteLocationNormalized, from: RouteLocationNormalized, next: NavigationGuardNext): NavigationGuardReturn | Promise<NavigationGuardReturn>;
}

export declare interface NavigationGuardNext {
    (): void;
    (error: Error): void;
    (location: RouteLocationRaw): void;
    (valid: boolean): void;
    (cb: NavigationGuardNextCallback): void;
}

declare type NavigationGuardNextCallback = (vm: ComponentPublicInstance) => any;

declare type NavigationGuardReturn = void | Error | RouteLocationRaw | boolean | NavigationGuardNextCallback;

declare interface NavigationGuardWithThis<T> {
    (this: T, to: RouteLocationNormalized, from: RouteLocationNormalized, next: NavigationGuardNext): NavigationGuardReturn | Promise<NavigationGuardReturn>;
}

declare interface NavigationInformation {
    type: NavigationType_2;
    direction: NavigationDirection;
    delta: number;
}

declare interface NavigationRedirectError extends Omit<NavigationFailure, 'to' | 'type'> {
    type: ErrorTypes.NAVIGATION_GUARD_REDIRECT;
    to: RouteLocationRaw;
}

declare enum NavigationType_2 {
    pop = "pop",
    push = "push"
}

/**
 * Add a navigation guard that triggers whenever the current location is
 * left. Similarly to {@link beforeRouteLeave}, it has access to the
 * component instance as `this`.
 *
 * @param leaveGuard - {@link NavigationGuard}
 */
export declare function onBeforeRouteLeave(leaveGuard: NavigationGuard): void;

/**
 * Add a navigation guard that triggers whenever the current location is
 * updated. Similarly to {@link beforeRouteUpdate}, it has access to the
 * component instance as `this`.
 *
 * @param updateGuard - {@link NavigationGuard}
 */
export declare function onBeforeRouteUpdate(updateGuard: NavigationGuard): void;

/**
 * Transforms a queryString into a {@link LocationQuery} object. Accept both, a
 * version with the leading `?` and without Should work as URLSearchParams
 *
 * @param search - search string to parse
 * @returns a query object
 */
export declare function parseQuery(search: string): LocationQuery;

declare type PathParams = Record<string, string | string[]>;

declare interface PathParser {
    /**
     * The regexp used to match a url
     */
    re: RegExp;
    /**
     * The score of the parser
     */
    score: Array<number[]>;
    /**
     * Keys that appeared in the path
     */
    keys: PathParserParamKey[];
    /**
     * Parses a url and returns the matched params or nul if it doesn't match. An
     * optional param that isn't preset will be an empty string. A repeatable
     * param will be an array if there is at least one value.
     *
     * @param path - url to parse
     * @returns a Params object, empty if there are no params. `null` if there is
     * no match
     */
    parse(path: string): PathParams | null;
    /**
     * Creates a string version of the url
     *
     * @param params - object of params
     * @returns a url
     */
    stringify(params: PathParams): string;
}

export declare type PathParserOptions = Pick<_PathParserOptions, 'end' | 'sensitive' | 'strict'>;

export declare interface _PathParserOptions {
    /**
     * Makes the RegExp case sensitive. Defaults to false
     */
    sensitive?: boolean;
    /**
     * Should we disallow a trailing slash. Defaults to false
     */
    strict?: boolean;
    /**
     * Should the RegExp match from the beginning by prepending a ^. Defaults to true
     */
    start?: boolean;
    /**
     * Should the RegExp match until the end by appending a $. Defaults to true
     */
    end?: boolean;
}

/**
 * A param in a url like `/users/:id`
 */
declare interface PathParserParamKey {
    name: string;
    repeatable: boolean;
    optional: boolean;
}

export declare interface PostNavigationGuard {
    (to: RouteLocationNormalized, from: RouteLocationNormalized, failure?: NavigationFailure | void): any;
}

declare type RawRouteComponent = RouteComponent | Lazy<RouteComponent>;

declare type RouteComponent = Component;

/**
 * {@link RouteLocationRaw} resolved using the matcher
 */
export declare interface RouteLocation extends _RouteLocationBase {
    /**
     * Array of {@link RouteRecord} containing components as they were
     * passed when adding records. It can also contain redirect records. This
     * can't be used directly
     */
    matched: RouteRecord[];
}

/**
 * Base properties for a normalized route location.
 *
 * @internal
 */
export declare interface _RouteLocationBase {
    /**
     * Percentage encoded pathname section of the URL.
     */
    path: string;
    /**
     * The whole location including the `search` and `hash`. This string is
     * percentage encoded.
     */
    fullPath: string;
    /**
     * Object representation of the `search` property of the current location.
     */
    query: LocationQuery;
    /**
     * Hash of the current location. If present, starts with a `#`.
     */
    hash: string;
    /**
     * Name of the matched record
     */
    name: RouteRecordName | null | undefined;
    /**
     * Object of decoded params extracted from the `path`.
     */
    params: RouteParams;
    /**
     * Contains the location we were initially trying to access before ending up
     * on the current location.
     */
    redirectedFrom: RouteLocation | undefined;
    /**
     * Merged `meta` properties from all of the matched route records.
     */
    meta: Record<string | number | symbol, any>;
}

export declare interface RouteLocationMatched extends RouteRecordNormalized {
    components: Record<string, RouteComponent>;
}

/**
 * Similar to {@link RouteLocation} but its
 * {@link RouteLocationNormalized.matched} cannot contain redirect records
 */
export declare interface RouteLocationNormalized extends _RouteLocationBase {
    /**
     * Array of {@link RouteRecordNormalized}
     */
    matched: RouteRecordNormalized[];
}

/**
 * {@link RouteLocationRaw} with
 */
export declare interface RouteLocationNormalizedLoaded extends _RouteLocationBase {
    /**
     * Array of {@link RouteLocationMatched} containing only plain components (any
     * lazy-loaded components have been loaded and were replaced inside of the
     * `components` object) so it can be directly used to display routes. It
     * cannot contain redirect records either
     */
    matched: RouteLocationMatched[];
}

export declare interface RouteLocationOptions {
    /**
     * Replace the entry in the history instead of pushing a new entry
     */
    replace?: boolean;
    /**
     * Triggers the navigation even if the location is the same as the current one
     */
    force?: boolean;
    /**
     * State to save using the History API. This cannot contain any reactive values and some primitives like Symbols are forbidden. More info at TODO: link mdn
     */
    state?: HistoryState;
}

/**
 * User-level route location
 */
export declare type RouteLocationRaw = string | (RouteQueryAndHash & LocationAsPath & RouteLocationOptions) | (RouteQueryAndHash & LocationAsNameRaw & RouteLocationOptions) | (RouteQueryAndHash & LocationAsRelativeRaw & RouteLocationOptions);

export declare type RouteParams = Record<string, RouteParamValue | RouteParamValue[]>;

declare type RouteParamsRaw = Record<string, RouteParamValueRaw | RouteParamValueRaw[]>;

declare type RouteParamValue = string;

declare type RouteParamValueRaw = RouteParamValue | number;

declare interface RouteQueryAndHash {
    query?: LocationQueryRaw;
    hash?: string;
}

export declare interface Router {
    /**
     * @internal
     */
    readonly currentRoute: Ref<RouteLocationNormalizedLoaded>;
    readonly options: RouterOptions;
    addRoute(parentName: RouteRecordName, route: RouteRecordRaw): () => void;
    addRoute(route: RouteRecordRaw): () => void;
    removeRoute(name: RouteRecordName): void;
    hasRoute(name: RouteRecordName): boolean;
    getRoutes(): RouteRecord[];
    resolve(to: RouteLocationRaw): RouteLocation & {
        href: string;
    };
    push(to: RouteLocationRaw): Promise<NavigationFailure | void | undefined>;
    replace(to: RouteLocationRaw): Promise<NavigationFailure | void | undefined>;
    back(): Promise<NavigationFailure | void | undefined>;
    forward(): Promise<NavigationFailure | void | undefined>;
    go(delta: number): Promise<NavigationFailure | void | undefined>;
    beforeEach(guard: NavigationGuardWithThis<undefined>): () => void;
    beforeResolve(guard: NavigationGuardWithThis<undefined>): () => void;
    afterEach(guard: PostNavigationGuard): () => void;
    onError(handler: ErrorHandler): () => void;
    isReady(): Promise<void>;
    install(app: App): void;
}

export declare type RouteRecord = RouteRecordNormalized;

/**
 * Common properties among all kind of {@link RouteRecordRaw}
 */
export declare interface _RouteRecordBase extends PathParserOptions {
    /**
     * Path of the record. Should start with `/` unless the record is the child of
     * another record.
     *
     * @example `/users/:id` matches `/users/1` as well as `/users/posva`.
     */
    path: string;
    /**
     * Where to redirect if the route is directly matched. The redirection happens
     * before any navigation guard and triggers a new navigation with the new
     * target location.
     */
    redirect?: RouteRecordRedirectOption;
    /**
     * Array of nested routes.
     */
    children?: RouteRecordRaw[];
    /**
     * Aliases for the record. Allows defining extra paths that will behave like a
     * copy of the record. Allows having paths shorthands like `/users/:id` and
     * `/u/:id`. All `alias` and `path` values must share the same params.
     */
    alias?: string | string[];
    /**
     * Name for the route record.
     */
    name?: RouteRecordName;
    /**
     * Before Enter guard specific to this record. Note `beforeEnter` has no
     * effect if the record has a `redirect` property.
     */
    beforeEnter?: NavigationGuardWithThis<undefined> | NavigationGuardWithThis<undefined>[];
    /**
     * Arbitrary data attached to the record.
     */
    meta?: Record<string | number | symbol, any>;
}

declare interface RouteRecordMatcher extends PathParser {
    record: RouteRecord;
    parent: RouteRecordMatcher | undefined;
    children: RouteRecordMatcher[];
    alias: RouteRecordMatcher[];
}

declare interface RouteRecordMultipleViews extends _RouteRecordBase {
    /**
     * Components to display when the URL matches this route. Allow using named views.
     */
    components: Record<string, RawRouteComponent>;
    /**
     * Allow passing down params as props to the component rendered by
     * `router-view`. Should be an object with the same keys as `components` or a
     * boolean to be applied to every component.
     */
    props?: Record<string, _RouteRecordProps> | boolean;
}

declare type RouteRecordName = string | symbol;

export declare interface RouteRecordNormalized {
    /**
     * {@inheritDoc _RouteRecordBase.path}
     */
    path: _RouteRecordBase['path'];
    /**
     * {@inheritDoc _RouteRecordBase.redirect}
     */
    redirect: _RouteRecordBase['redirect'] | undefined;
    /**
     * {@inheritDoc _RouteRecordBase.name}
     */
    name: _RouteRecordBase['name'];
    /**
     * {@inheritDoc RouteRecordMultipleViews.components}
     */
    components: RouteRecordMultipleViews['components'];
    /**
     * {@inheritDoc _RouteRecordBase.components}
     */
    children: Exclude<_RouteRecordBase['children'], void>;
    /**
     * {@inheritDoc _RouteRecordBase.meta}
     */
    meta: Exclude<_RouteRecordBase['meta'], void>;
    /**
     * {@inheritDoc RouteRecordMultipleViews.props}
     */
    props: Record<string, _RouteRecordProps>;
    /**
     * {@inheritDoc _RouteRecordBase.props}
     */
    beforeEnter: RouteRecordMultipleViews['beforeEnter'];
    /**
     * Registered leave guards
     *
     * @internal
     */
    leaveGuards: NavigationGuard[];
    /**
     * Registered update guards
     *
     * @internal
     */
    updateGuards: NavigationGuard[];
    /**
     * Registered beforeRouteEnter callbacks passed to `next` or returned in guards
     *
     * @internal
     */
    enterCallbacks: Record<string, NavigationGuardNextCallback[]>;
    /**
     * Mounted route component instances
     * Having the instances on the record mean beforeRouteUpdate and
     * beforeRouteLeave guards can only be invoked with the latest mounted app
     * instance if there are multiple application instances rendering the same
     * view, basically duplicating the content on the page, which shouldn't happen
     * in practice. It will work if multiple apps are rendering different named
     * views.
     */
    instances: Record<string, ComponentPublicInstance | undefined | null>;
    /**
     * Defines if this record is the alias of another one. This property is
     * `undefined` if the record is the original one.
     */
    aliasOf: RouteRecordNormalized | undefined;
}

/**
 * @internal
 */
declare type _RouteRecordProps = boolean | Record<string, any> | ((to: RouteLocationNormalized) => Record<string, any>);

export declare type RouteRecordRaw = RouteRecordSingleView | RouteRecordMultipleViews | RouteRecordRedirect;

declare interface RouteRecordRedirect extends _RouteRecordBase {
    redirect: RouteRecordRedirectOption;
    component?: never;
    components?: never;
    children?: never;
}

declare type RouteRecordRedirectOption = RouteLocationRaw | ((to: RouteLocation) => RouteLocationRaw);

declare interface RouteRecordSingleView extends _RouteRecordBase {
    /**
     * Component to display when the URL matches this route.
     */
    component: RawRouteComponent;
    /**
     * Allow passing down params as props to the component rendered by `router-view`.
     */
    props?: _RouteRecordProps;
}

declare interface RouterErrorBase extends Error {
    type: ErrorTypes;
}

/**
 * Interface implemented by History implementations that can be passed to the
 * router as {@link Router.history}
 *
 * @alpha
 */
export declare interface RouterHistory {
    /**
     * Base path that is prepended to every url. This allows hosting an SPA at a
     * subfolder of a domain like `example.com/subfolder` by having a `base` of
     * `/subfolder`
     */
    readonly base: string;
    /**
     * Current History location
     */
    readonly location: HistoryLocation;
    /**
     * Current History state
     */
    readonly state: HistoryState;
    /**
     * Navigates to a location. In the case of an HTML5 History implementation,
     * this will call `history.pushState` to effectively change the URL.
     *
     * @param to - location to push
     * @param data - optional {@link HistoryState} to be associated with the
     * navigation entry
     */
    push(to: HistoryLocation, data?: HistoryState): void;
    /**
     * Same as {@link RouterHistory.push} but performs a `history.replaceState`
     * instead of `history.pushState`
     *
     * @param to - location to set
     * @param data - optional {@link HistoryState} to be associated with the
     * navigation entry
     */
    replace(to: HistoryLocation, data?: HistoryState): void;
    /**
     * Traverses history in a given direction.
     *
     * @example
     * ```js
     * myHistory.go(-1) // equivalent to window.history.back()
     * myHistory.go(1) // equivalent to window.history.forward()
     * ```
     *
     * @param delta - distance to travel. If delta is \< 0, it will go back,
     * if it's \> 0, it will go forward by that amount of entries.
     * @param triggerListeners - whether this should trigger listeners attached to
     * the history
     */
    go(delta: number, triggerListeners?: boolean): void;
    /**
     * Attach a listener to the History implementation that is triggered when the
     * navigation is triggered from outside (like the Browser back and forward
     * buttons) or when passing `true` to {@link RouterHistory.back} and
     * {@link RouterHistory.forward}
     *
     * @param callback - listener to attach
     * @returns a callback to remove the listener
     */
    listen(callback: NavigationCallback): () => void;
    /**
     * Generates the corresponding href to be used in an anchor tag.
     *
     * @param location - history location that should create an href
     */
    createHref(location: HistoryLocation): string;
    /**
     * Clears any event listener attached by the history implementation.
     */
    destroy(): void;
}

export declare const RouterLink: new () => {
    $props: VNodeProps & RouterLinkProps;
};

declare interface RouterLinkOptions {
    /**
     * Location the link should navigate to when clicked on.
     */
    to: RouteLocationRaw;
    /**
     * Calls `router.replace` instead of `router.push`.
     */
    replace?: boolean;
}

export declare interface RouterLinkProps extends RouterLinkOptions {
    /**
     * Whether RouterLink should not wrap its content in an `a` tag.
     */
    custom?: boolean;
    /**
     * Class to apply when the link is active
     */
    activeClass?: string;
    /**
     * Class to apply when the link is exact active
     */
    exactActiveClass?: string;
    /**
     * Value passed to the attribute `aria-current` when the link is exact active. Defaults to "page"
     */
    ariaCurrentValue?: 'page' | 'step' | 'location' | 'date' | 'time' | 'true' | 'false';
}

declare interface RouterMatcher {
    addRoute: (record: RouteRecordRaw, parent?: RouteRecordMatcher) => () => void;
    removeRoute: {
        (matcher: RouteRecordMatcher): void;
        (name: RouteRecordName): void;
    };
    getRoutes: () => RouteRecordMatcher[];
    getRecordMatcher: (name: RouteRecordName) => RouteRecordMatcher | undefined;
    /**
     * Resolves a location. Gives access to the route record that corresponds to the actual path as well as filling the corresponding params objects
     *
     * @param location - MatcherLocationRaw to resolve to a url
     * @param currentLocation - MatcherLocation of the current location
     */
    resolve: (location: MatcherLocationRaw, currentLocation: MatcherLocation) => MatcherLocation;
}

export declare interface RouterOptions extends PathParserOptions {
    /**
     * History implementation used by the router. Most web applications should use
     * `createWebHistory` but it requires the server to be properly configured.
     * You can also use a _hash_ based history with `createWebHashHistory` that
     * does not require any configuration on the server but isn't handled at all
     * by search engines and does poorly on SEO.
     *
     * @example
     * ```js
     * createRouter({
     *   history: createWebHistory(),
     *   // other options...
     * })
     * ```
     */
    history: RouterHistory;
    /**
     * Initial list of routes that should be added to the router.
     */
    routes: RouteRecordRaw[];
    /**
     * Function to control scrolling when navigating between pages.
     */
    scrollBehavior?: ScrollBehavior_2;
    /**
     * Custom implementation to parse a query.
     *
     * @example
     * Let's say you want to use the package {@link https://github.com/ljharb/qs | `qs`}
     * to parse queries, you would need to provide both `parseQuery` and
     * {@link RouterOptions.stringifyQuery | `stringifyQuery`}:
     * ```js
     * import qs from 'qs'
     *
     * createRouter({
     *   // other options...
     *   parse: qs.parse,
     *   stringifyQuery: qs.stringify,
     * })
     * ```
     */
    parseQuery?: typeof parseQuery;
    /**
     * {@link RouterOptions.parseQuery | `parseQuery`} counterpart to handle query parsing.
     */
    stringifyQuery?: typeof stringifyQuery;
    /**
     * Default class applied to active {@link RouterLink}. If none is provided,
     * `router-link-active` will be applied.
     */
    linkActiveClass?: string;
    /**
     * Default class applied to exact active {@link RouterLink}. If none is provided,
     * `router-link-exact-active` will be applied.
     */
    linkExactActiveClass?: string;
}

export declare const RouterView: new () => {
    $props: VNodeProps & RouterViewProps;
};

export declare interface RouterViewProps {
    name?: string;
    route?: RouteLocationNormalized;
}

declare interface ScrollBehavior_2 {
    (to: RouteLocationNormalized, from: RouteLocationNormalizedLoaded, savedPosition: _ScrollPositionNormalized | null): Awaitable<ScrollPosition | false | void>;
}
export { ScrollBehavior_2 as ScrollBehavior }

declare type ScrollPosition = ScrollPositionCoordinates | ScrollPositionElement;

/**
 * Scroll position similar to
 * {@link https://developer.mozilla.org/en-US/docs/Web/API/ScrollToOptions | `ScrollToOptions`}.
 * Note that not all browsers support `behavior`.
 */
declare type ScrollPositionCoordinates = {
    behavior?: ScrollOptions['behavior'];
    left?: number;
    top?: number;
};

declare interface ScrollPositionElement extends ScrollToOptions {
    /**
     * A valid CSS selector. Note some characters must be escaped in id selectors (https://mathiasbynens.be/notes/css-escapes).
     * @example
     * Here are a few examples:
     *
     * - `.title`
     * - `.content:first-child`
     * - `#marker`
     * - `#marker\~with\~symbols`
     * - `#marker.with.dot`: selects `class="with dot" id="marker"`, not `id="marker.with.dot"`
     *
     */
    el: string | Element;
}

/**
 * Internal normalized version of {@link ScrollPositionCoordinates} that always
 * has `left` and `top` coordinates.
 *
 * @internal
 */
declare type _ScrollPositionNormalized = {
    behavior?: ScrollOptions['behavior'];
    left: number;
    top: number;
};

export declare const START_LOCATION: RouteLocationNormalizedLoaded;

/**
 * Stringifies a {@link LocationQueryRaw} object. Like `URLSearchParams`, it
 * doesn't prepend a `?`
 *
 * @param query - query object to stringify
 * @returns string version of the query without the leading `?`
 */
export declare function stringifyQuery(query: LocationQueryRaw): string;

export declare function useLink(props: UseLinkOptions): {
    route: import("vue").ComputedRef<RouteLocation & {
        href: string;
    }>;
    href: import("vue").ComputedRef<string>;
    isActive: import("vue").ComputedRef<boolean>;
    isExactActive: import("vue").ComputedRef<boolean>;
    navigate: (e?: MouseEvent) => Promise<any>;
};

declare type UseLinkOptions = VueUseOptions<RouterLinkOptions>;

export declare function useRoute(): RouteLocationNormalizedLoaded;

export declare function useRouter(): Router;

declare type VueUseOptions<T> = {
    [k in keyof T]: Ref<T[k]> | T[k] | ComputedRef<T[k]>;
};

export { }

declare module 'vue' {
  export interface ComponentCustomOptions {
    /**
     * Guard called when the router is navigating to the route that is rendering
     * this component from a different route. Differently from `beforeRouteUpdate`
     * and `beforeRouteLeave`, `beforeRouteEnter` does not have access to the
     * component instance through `this` because it triggers before the component
     * is even mounted.
     *
     * @param to - RouteLocationRaw we are navigating to
     * @param from - RouteLocationRaw we are navigating from
     * @param next - function to validate, cancel or modify (by redirecting) the
     * navigation
     */
    beforeRouteEnter?: NavigationGuardWithThis<undefined>

    /**
     * Guard called whenever the route that renders this component has changed but
     * it is reused for the new route. This allows you to guard for changes in
     * params, the query or the hash.
     *
     * @param to - RouteLocationRaw we are navigating to
     * @param from - RouteLocationRaw we are navigating from
     * @param next - function to validate, cancel or modify (by redirecting) the
     * navigation
     */
    beforeRouteUpdate?: NavigationGuard

    /**
     * Guard called when the router is navigating away from the current route that
     * is rendering this component.
     *
     * @param to - RouteLocationRaw we are navigating to
     * @param from - RouteLocationRaw we are navigating from
     * @param next - function to validate, cancel or modify (by redirecting) the
     * navigation
     */
    beforeRouteLeave?: NavigationGuard
  }

  export interface ComponentCustomProperties {
    /**
     * Normalized current location. See {@link RouteLocationNormalizedLoaded}.
     */
    $route: RouteLocationNormalizedLoaded
    /**
     * {@link Router} instance used by the application.
     */
    $router: Router
  }
}
