// Query parameters and path variables of a URL as typed, for the params
// table (the same rules as `ac_domain::params` on the Rust side). Values
// stay as typed – no percent-decoding – so `{{variables}}` survive.

/** A name and a value. */
export interface Param {
  name: string;
  value: string;
}

/** The URL split into the part before the query, the query and the fragment (with `#`). */
function split(url: string): { base: string; query: string | null; fragment: string } {
  const hash = url.indexOf('#');
  const fragment = hash < 0 ? '' : url.slice(hash);
  const rest = hash < 0 ? url : url.slice(0, hash);
  const question = rest.indexOf('?');
  return question < 0
    ? { base: rest, query: null, fragment }
    : { base: rest.slice(0, question), query: rest.slice(question + 1), fragment };
}

/** The query parameters in `url`, in order. */
export function queryParams(url: string): Param[] {
  const { query } = split(url);
  if (query === null) return [];
  return query
    .split('&')
    .filter((part) => part !== '')
    .map((part) => {
      const equals = part.indexOf('=');
      return equals < 0
        ? { name: part, value: '' }
        : { name: part.slice(0, equals), value: part.slice(equals + 1) };
    });
}

/** `url` with its query replaced by `params` (rows without a name are left out). */
export function withQueryParams(url: string, params: Param[]): string {
  const { base, fragment } = split(url);
  const query = params
    .filter((param) => param.name !== '')
    .map((param) => (param.value === '' ? param.name : `${param.name}=${param.value}`))
    .join('&');
  return `${base}${query === '' ? '' : `?${query}`}${fragment}`;
}

const PATH_VARIABLE = /\/:([A-Za-z0-9_-]+)/g;

/** Names of the path variables (`/:id`) in the path of `url`, each once. */
export function pathParamNames(url: string): string[] {
  const { base } = split(url);
  const names: string[] = [];
  for (const match of base.matchAll(PATH_VARIABLE)) {
    const name = match[1]!;
    if (!names.includes(name)) names.push(name);
  }
  return names;
}

/** `url` with the path variables that have a value replaced (for the preview). */
export function fillPathParams(url: string, params: Param[]): string {
  const { base, query, fragment } = split(url);
  const filled = base.replace(PATH_VARIABLE, (whole, name: string) => {
    const param = params.find((candidate) => candidate.name === name);
    return param && param.value !== '' ? `/${param.value}` : whole;
  });
  return `${filled}${query === null ? '' : `?${query}`}${fragment}`;
}
