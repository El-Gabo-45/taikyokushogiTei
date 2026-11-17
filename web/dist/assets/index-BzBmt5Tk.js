//#region \0vite/modulepreload-polyfill.js
(function polyfill() {
	const relList = document.createElement("link").relList;
	if (relList && relList.supports && relList.supports("modulepreload")) return;
	for (const link of document.querySelectorAll("link[rel=\"modulepreload\"]")) processPreload(link);
	new MutationObserver((mutations) => {
		for (const mutation of mutations) {
			if (mutation.type !== "childList") continue;
			for (const node of mutation.addedNodes) if (node.tagName === "LINK" && node.rel === "modulepreload") processPreload(node);
		}
	}).observe(document, {
		childList: true,
		subtree: true
	});
	function getFetchOpts(link) {
		const fetchOpts = {};
		if (link.integrity) fetchOpts.integrity = link.integrity;
		if (link.referrerPolicy) fetchOpts.referrerPolicy = link.referrerPolicy;
		if (link.crossOrigin === "use-credentials") fetchOpts.credentials = "include";
		else if (link.crossOrigin === "anonymous") fetchOpts.credentials = "omit";
		else fetchOpts.credentials = "same-origin";
		return fetchOpts;
	}
	function processPreload(link) {
		if (link.ep) return;
		link.ep = true;
		const fetchOpts = getFetchOpts(link);
		fetch(link.href, fetchOpts);
	}
})();
//#endregion
//#region node_modules/zod/v3/helpers/util.js
var util;
(function(util) {
	util.assertEqual = (_) => {};
	function assertIs(_arg) {}
	util.assertIs = assertIs;
	function assertNever(_x) {
		throw new Error();
	}
	util.assertNever = assertNever;
	util.arrayToEnum = (items) => {
		const obj = {};
		for (const item of items) obj[item] = item;
		return obj;
	};
	util.getValidEnumValues = (obj) => {
		const validKeys = util.objectKeys(obj).filter((k) => typeof obj[obj[k]] !== "number");
		const filtered = {};
		for (const k of validKeys) filtered[k] = obj[k];
		return util.objectValues(filtered);
	};
	util.objectValues = (obj) => {
		return util.objectKeys(obj).map(function(e) {
			return obj[e];
		});
	};
	util.objectKeys = typeof Object.keys === "function" ? (obj) => Object.keys(obj) : (object) => {
		const keys = [];
		for (const key in object) if (Object.prototype.hasOwnProperty.call(object, key)) keys.push(key);
		return keys;
	};
	util.find = (arr, checker) => {
		for (const item of arr) if (checker(item)) return item;
	};
	util.isInteger = typeof Number.isInteger === "function" ? (val) => Number.isInteger(val) : (val) => typeof val === "number" && Number.isFinite(val) && Math.floor(val) === val;
	function joinValues(array, separator = " | ") {
		return array.map((val) => typeof val === "string" ? `'${val}'` : val).join(separator);
	}
	util.joinValues = joinValues;
	util.jsonStringifyReplacer = (_, value) => {
		if (typeof value === "bigint") return value.toString();
		return value;
	};
})(util || (util = {}));
var objectUtil;
(function(objectUtil) {
	objectUtil.mergeShapes = (first, second) => {
		return {
			...first,
			...second
		};
	};
})(objectUtil || (objectUtil = {}));
var ZodParsedType = util.arrayToEnum([
	"string",
	"nan",
	"number",
	"integer",
	"float",
	"boolean",
	"date",
	"bigint",
	"symbol",
	"function",
	"undefined",
	"null",
	"array",
	"object",
	"unknown",
	"promise",
	"void",
	"never",
	"map",
	"set"
]);
var getParsedType = (data) => {
	switch (typeof data) {
		case "undefined": return ZodParsedType.undefined;
		case "string": return ZodParsedType.string;
		case "number": return Number.isNaN(data) ? ZodParsedType.nan : ZodParsedType.number;
		case "boolean": return ZodParsedType.boolean;
		case "function": return ZodParsedType.function;
		case "bigint": return ZodParsedType.bigint;
		case "symbol": return ZodParsedType.symbol;
		case "object":
			if (Array.isArray(data)) return ZodParsedType.array;
			if (data === null) return ZodParsedType.null;
			if (data.then && typeof data.then === "function" && data.catch && typeof data.catch === "function") return ZodParsedType.promise;
			if (typeof Map !== "undefined" && data instanceof Map) return ZodParsedType.map;
			if (typeof Set !== "undefined" && data instanceof Set) return ZodParsedType.set;
			if (typeof Date !== "undefined" && data instanceof Date) return ZodParsedType.date;
			return ZodParsedType.object;
		default: return ZodParsedType.unknown;
	}
};
//#endregion
//#region node_modules/zod/v3/ZodError.js
var ZodIssueCode = util.arrayToEnum([
	"invalid_type",
	"invalid_literal",
	"custom",
	"invalid_union",
	"invalid_union_discriminator",
	"invalid_enum_value",
	"unrecognized_keys",
	"invalid_arguments",
	"invalid_return_type",
	"invalid_date",
	"invalid_string",
	"too_small",
	"too_big",
	"invalid_intersection_types",
	"not_multiple_of",
	"not_finite"
]);
var ZodError = class ZodError extends Error {
	get errors() {
		return this.issues;
	}
	constructor(issues) {
		super();
		this.issues = [];
		this.addIssue = (sub) => {
			this.issues = [...this.issues, sub];
		};
		this.addIssues = (subs = []) => {
			this.issues = [...this.issues, ...subs];
		};
		const actualProto = new.target.prototype;
		if (Object.setPrototypeOf) Object.setPrototypeOf(this, actualProto);
		else this.__proto__ = actualProto;
		this.name = "ZodError";
		this.issues = issues;
	}
	format(_mapper) {
		const mapper = _mapper || function(issue) {
			return issue.message;
		};
		const fieldErrors = { _errors: [] };
		const processError = (error) => {
			for (const issue of error.issues) if (issue.code === "invalid_union") issue.unionErrors.map(processError);
			else if (issue.code === "invalid_return_type") processError(issue.returnTypeError);
			else if (issue.code === "invalid_arguments") processError(issue.argumentsError);
			else if (issue.path.length === 0) fieldErrors._errors.push(mapper(issue));
			else {
				let curr = fieldErrors;
				let i = 0;
				while (i < issue.path.length) {
					const el = issue.path[i];
					if (!(i === issue.path.length - 1)) curr[el] = curr[el] || { _errors: [] };
					else {
						curr[el] = curr[el] || { _errors: [] };
						curr[el]._errors.push(mapper(issue));
					}
					curr = curr[el];
					i++;
				}
			}
		};
		processError(this);
		return fieldErrors;
	}
	static assert(value) {
		if (!(value instanceof ZodError)) throw new Error(`Not a ZodError: ${value}`);
	}
	toString() {
		return this.message;
	}
	get message() {
		return JSON.stringify(this.issues, util.jsonStringifyReplacer, 2);
	}
	get isEmpty() {
		return this.issues.length === 0;
	}
	flatten(mapper = (issue) => issue.message) {
		const fieldErrors = {};
		const formErrors = [];
		for (const sub of this.issues) if (sub.path.length > 0) {
			const firstEl = sub.path[0];
			fieldErrors[firstEl] = fieldErrors[firstEl] || [];
			fieldErrors[firstEl].push(mapper(sub));
		} else formErrors.push(mapper(sub));
		return {
			formErrors,
			fieldErrors
		};
	}
	get formErrors() {
		return this.flatten();
	}
};
ZodError.create = (issues) => {
	return new ZodError(issues);
};
//#endregion
//#region node_modules/zod/v3/locales/en.js
var errorMap = (issue, _ctx) => {
	let message;
	switch (issue.code) {
		case ZodIssueCode.invalid_type:
			if (issue.received === ZodParsedType.undefined) message = "Required";
			else message = `Expected ${issue.expected}, received ${issue.received}`;
			break;
		case ZodIssueCode.invalid_literal:
			message = `Invalid literal value, expected ${JSON.stringify(issue.expected, util.jsonStringifyReplacer)}`;
			break;
		case ZodIssueCode.unrecognized_keys:
			message = `Unrecognized key(s) in object: ${util.joinValues(issue.keys, ", ")}`;
			break;
		case ZodIssueCode.invalid_union:
			message = `Invalid input`;
			break;
		case ZodIssueCode.invalid_union_discriminator:
			message = `Invalid discriminator value. Expected ${util.joinValues(issue.options)}`;
			break;
		case ZodIssueCode.invalid_enum_value:
			message = `Invalid enum value. Expected ${util.joinValues(issue.options)}, received '${issue.received}'`;
			break;
		case ZodIssueCode.invalid_arguments:
			message = `Invalid function arguments`;
			break;
		case ZodIssueCode.invalid_return_type:
			message = `Invalid function return type`;
			break;
		case ZodIssueCode.invalid_date:
			message = `Invalid date`;
			break;
		case ZodIssueCode.invalid_string:
			if (typeof issue.validation === "object") if ("includes" in issue.validation) {
				message = `Invalid input: must include "${issue.validation.includes}"`;
				if (typeof issue.validation.position === "number") message = `${message} at one or more positions greater than or equal to ${issue.validation.position}`;
			} else if ("startsWith" in issue.validation) message = `Invalid input: must start with "${issue.validation.startsWith}"`;
			else if ("endsWith" in issue.validation) message = `Invalid input: must end with "${issue.validation.endsWith}"`;
			else util.assertNever(issue.validation);
			else if (issue.validation !== "regex") message = `Invalid ${issue.validation}`;
			else message = "Invalid";
			break;
		case ZodIssueCode.too_small:
			if (issue.type === "array") message = `Array must contain ${issue.exact ? "exactly" : issue.inclusive ? `at least` : `more than`} ${issue.minimum} element(s)`;
			else if (issue.type === "string") message = `String must contain ${issue.exact ? "exactly" : issue.inclusive ? `at least` : `over`} ${issue.minimum} character(s)`;
			else if (issue.type === "number") message = `Number must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${issue.minimum}`;
			else if (issue.type === "bigint") message = `Number must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${issue.minimum}`;
			else if (issue.type === "date") message = `Date must be ${issue.exact ? `exactly equal to ` : issue.inclusive ? `greater than or equal to ` : `greater than `}${new Date(Number(issue.minimum))}`;
			else message = "Invalid input";
			break;
		case ZodIssueCode.too_big:
			if (issue.type === "array") message = `Array must contain ${issue.exact ? `exactly` : issue.inclusive ? `at most` : `less than`} ${issue.maximum} element(s)`;
			else if (issue.type === "string") message = `String must contain ${issue.exact ? `exactly` : issue.inclusive ? `at most` : `under`} ${issue.maximum} character(s)`;
			else if (issue.type === "number") message = `Number must be ${issue.exact ? `exactly` : issue.inclusive ? `less than or equal to` : `less than`} ${issue.maximum}`;
			else if (issue.type === "bigint") message = `BigInt must be ${issue.exact ? `exactly` : issue.inclusive ? `less than or equal to` : `less than`} ${issue.maximum}`;
			else if (issue.type === "date") message = `Date must be ${issue.exact ? `exactly` : issue.inclusive ? `smaller than or equal to` : `smaller than`} ${new Date(Number(issue.maximum))}`;
			else message = "Invalid input";
			break;
		case ZodIssueCode.custom:
			message = `Invalid input`;
			break;
		case ZodIssueCode.invalid_intersection_types:
			message = `Intersection results could not be merged`;
			break;
		case ZodIssueCode.not_multiple_of:
			message = `Number must be a multiple of ${issue.multipleOf}`;
			break;
		case ZodIssueCode.not_finite:
			message = "Number must be finite";
			break;
		default:
			message = _ctx.defaultError;
			util.assertNever(issue);
	}
	return { message };
};
//#endregion
//#region node_modules/zod/v3/errors.js
var overrideErrorMap = errorMap;
function getErrorMap() {
	return overrideErrorMap;
}
//#endregion
//#region node_modules/zod/v3/helpers/parseUtil.js
var makeIssue = (params) => {
	const { data, path, errorMaps, issueData } = params;
	const fullPath = [...path, ...issueData.path || []];
	const fullIssue = {
		...issueData,
		path: fullPath
	};
	if (issueData.message !== void 0) return {
		...issueData,
		path: fullPath,
		message: issueData.message
	};
	let errorMessage = "";
	const maps = errorMaps.filter((m) => !!m).slice().reverse();
	for (const map of maps) errorMessage = map(fullIssue, {
		data,
		defaultError: errorMessage
	}).message;
	return {
		...issueData,
		path: fullPath,
		message: errorMessage
	};
};
function addIssueToContext(ctx, issueData) {
	const overrideMap = getErrorMap();
	const issue = makeIssue({
		issueData,
		data: ctx.data,
		path: ctx.path,
		errorMaps: [
			ctx.common.contextualErrorMap,
			ctx.schemaErrorMap,
			overrideMap,
			overrideMap === errorMap ? void 0 : errorMap
		].filter((x) => !!x)
	});
	ctx.common.issues.push(issue);
}
var ParseStatus = class ParseStatus {
	constructor() {
		this.value = "valid";
	}
	dirty() {
		if (this.value === "valid") this.value = "dirty";
	}
	abort() {
		if (this.value !== "aborted") this.value = "aborted";
	}
	static mergeArray(status, results) {
		const arrayValue = [];
		for (const s of results) {
			if (s.status === "aborted") return INVALID;
			if (s.status === "dirty") status.dirty();
			arrayValue.push(s.value);
		}
		return {
			status: status.value,
			value: arrayValue
		};
	}
	static async mergeObjectAsync(status, pairs) {
		const syncPairs = [];
		for (const pair of pairs) {
			const key = await pair.key;
			const value = await pair.value;
			syncPairs.push({
				key,
				value
			});
		}
		return ParseStatus.mergeObjectSync(status, syncPairs);
	}
	static mergeObjectSync(status, pairs) {
		const finalObject = {};
		for (const pair of pairs) {
			const { key, value } = pair;
			if (key.status === "aborted") return INVALID;
			if (value.status === "aborted") return INVALID;
			if (key.status === "dirty") status.dirty();
			if (value.status === "dirty") status.dirty();
			if (key.value !== "__proto__" && (typeof value.value !== "undefined" || pair.alwaysSet)) finalObject[key.value] = value.value;
		}
		return {
			status: status.value,
			value: finalObject
		};
	}
};
var INVALID = Object.freeze({ status: "aborted" });
var DIRTY = (value) => ({
	status: "dirty",
	value
});
var OK = (value) => ({
	status: "valid",
	value
});
var isAborted = (x) => x.status === "aborted";
var isDirty = (x) => x.status === "dirty";
var isValid = (x) => x.status === "valid";
var isAsync = (x) => typeof Promise !== "undefined" && x instanceof Promise;
//#endregion
//#region node_modules/zod/v3/helpers/errorUtil.js
var errorUtil;
(function(errorUtil) {
	errorUtil.errToObj = (message) => typeof message === "string" ? { message } : message || {};
	errorUtil.toString = (message) => typeof message === "string" ? message : message?.message;
})(errorUtil || (errorUtil = {}));
//#endregion
//#region node_modules/zod/v3/types.js
var ParseInputLazyPath = class {
	constructor(parent, value, path, key) {
		this._cachedPath = [];
		this.parent = parent;
		this.data = value;
		this._path = path;
		this._key = key;
	}
	get path() {
		if (!this._cachedPath.length) if (Array.isArray(this._key)) this._cachedPath.push(...this._path, ...this._key);
		else this._cachedPath.push(...this._path, this._key);
		return this._cachedPath;
	}
};
var handleResult = (ctx, result) => {
	if (isValid(result)) return {
		success: true,
		data: result.value
	};
	else {
		if (!ctx.common.issues.length) throw new Error("Validation failed but no issues detected.");
		return {
			success: false,
			get error() {
				if (this._error) return this._error;
				const error = new ZodError(ctx.common.issues);
				this._error = error;
				return this._error;
			}
		};
	}
};
function processCreateParams(params) {
	if (!params) return {};
	const { errorMap, invalid_type_error, required_error, description } = params;
	if (errorMap && (invalid_type_error || required_error)) throw new Error(`Can't use "invalid_type_error" or "required_error" in conjunction with custom error map.`);
	if (errorMap) return {
		errorMap,
		description
	};
	const customMap = (iss, ctx) => {
		const { message } = params;
		if (iss.code === "invalid_enum_value") return { message: message ?? ctx.defaultError };
		if (typeof ctx.data === "undefined") return { message: message ?? required_error ?? ctx.defaultError };
		if (iss.code !== "invalid_type") return { message: ctx.defaultError };
		return { message: message ?? invalid_type_error ?? ctx.defaultError };
	};
	return {
		errorMap: customMap,
		description
	};
}
var ZodType = class {
	get description() {
		return this._def.description;
	}
	_getType(input) {
		return getParsedType(input.data);
	}
	_getOrReturnCtx(input, ctx) {
		return ctx || {
			common: input.parent.common,
			data: input.data,
			parsedType: getParsedType(input.data),
			schemaErrorMap: this._def.errorMap,
			path: input.path,
			parent: input.parent
		};
	}
	_processInputParams(input) {
		return {
			status: new ParseStatus(),
			ctx: {
				common: input.parent.common,
				data: input.data,
				parsedType: getParsedType(input.data),
				schemaErrorMap: this._def.errorMap,
				path: input.path,
				parent: input.parent
			}
		};
	}
	_parseSync(input) {
		const result = this._parse(input);
		if (isAsync(result)) throw new Error("Synchronous parse encountered promise.");
		return result;
	}
	_parseAsync(input) {
		const result = this._parse(input);
		return Promise.resolve(result);
	}
	parse(data, params) {
		const result = this.safeParse(data, params);
		if (result.success) return result.data;
		throw result.error;
	}
	safeParse(data, params) {
		const ctx = {
			common: {
				issues: [],
				async: params?.async ?? false,
				contextualErrorMap: params?.errorMap
			},
			path: params?.path || [],
			schemaErrorMap: this._def.errorMap,
			parent: null,
			data,
			parsedType: getParsedType(data)
		};
		return handleResult(ctx, this._parseSync({
			data,
			path: ctx.path,
			parent: ctx
		}));
	}
	"~validate"(data) {
		const ctx = {
			common: {
				issues: [],
				async: !!this["~standard"].async
			},
			path: [],
			schemaErrorMap: this._def.errorMap,
			parent: null,
			data,
			parsedType: getParsedType(data)
		};
		if (!this["~standard"].async) try {
			const result = this._parseSync({
				data,
				path: [],
				parent: ctx
			});
			return isValid(result) ? { value: result.value } : { issues: ctx.common.issues };
		} catch (err) {
			if (err?.message?.toLowerCase()?.includes("encountered")) this["~standard"].async = true;
			ctx.common = {
				issues: [],
				async: true
			};
		}
		return this._parseAsync({
			data,
			path: [],
			parent: ctx
		}).then((result) => isValid(result) ? { value: result.value } : { issues: ctx.common.issues });
	}
	async parseAsync(data, params) {
		const result = await this.safeParseAsync(data, params);
		if (result.success) return result.data;
		throw result.error;
	}
	async safeParseAsync(data, params) {
		const ctx = {
			common: {
				issues: [],
				contextualErrorMap: params?.errorMap,
				async: true
			},
			path: params?.path || [],
			schemaErrorMap: this._def.errorMap,
			parent: null,
			data,
			parsedType: getParsedType(data)
		};
		const maybeAsyncResult = this._parse({
			data,
			path: ctx.path,
			parent: ctx
		});
		return handleResult(ctx, await (isAsync(maybeAsyncResult) ? maybeAsyncResult : Promise.resolve(maybeAsyncResult)));
	}
	refine(check, message) {
		const getIssueProperties = (val) => {
			if (typeof message === "string" || typeof message === "undefined") return { message };
			else if (typeof message === "function") return message(val);
			else return message;
		};
		return this._refinement((val, ctx) => {
			const result = check(val);
			const setError = () => ctx.addIssue({
				code: ZodIssueCode.custom,
				...getIssueProperties(val)
			});
			if (typeof Promise !== "undefined" && result instanceof Promise) return result.then((data) => {
				if (!data) {
					setError();
					return false;
				} else return true;
			});
			if (!result) {
				setError();
				return false;
			} else return true;
		});
	}
	refinement(check, refinementData) {
		return this._refinement((val, ctx) => {
			if (!check(val)) {
				ctx.addIssue(typeof refinementData === "function" ? refinementData(val, ctx) : refinementData);
				return false;
			} else return true;
		});
	}
	_refinement(refinement) {
		return new ZodEffects({
			schema: this,
			typeName: ZodFirstPartyTypeKind.ZodEffects,
			effect: {
				type: "refinement",
				refinement
			}
		});
	}
	superRefine(refinement) {
		return this._refinement(refinement);
	}
	constructor(def) {
		/** Alias of safeParseAsync */
		this.spa = this.safeParseAsync;
		this._def = def;
		this.parse = this.parse.bind(this);
		this.safeParse = this.safeParse.bind(this);
		this.parseAsync = this.parseAsync.bind(this);
		this.safeParseAsync = this.safeParseAsync.bind(this);
		this.spa = this.spa.bind(this);
		this.refine = this.refine.bind(this);
		this.refinement = this.refinement.bind(this);
		this.superRefine = this.superRefine.bind(this);
		this.optional = this.optional.bind(this);
		this.nullable = this.nullable.bind(this);
		this.nullish = this.nullish.bind(this);
		this.array = this.array.bind(this);
		this.promise = this.promise.bind(this);
		this.or = this.or.bind(this);
		this.and = this.and.bind(this);
		this.transform = this.transform.bind(this);
		this.brand = this.brand.bind(this);
		this.default = this.default.bind(this);
		this.catch = this.catch.bind(this);
		this.describe = this.describe.bind(this);
		this.pipe = this.pipe.bind(this);
		this.readonly = this.readonly.bind(this);
		this.isNullable = this.isNullable.bind(this);
		this.isOptional = this.isOptional.bind(this);
		this["~standard"] = {
			version: 1,
			vendor: "zod",
			validate: (data) => this["~validate"](data)
		};
	}
	optional() {
		return ZodOptional.create(this, this._def);
	}
	nullable() {
		return ZodNullable.create(this, this._def);
	}
	nullish() {
		return this.nullable().optional();
	}
	array() {
		return ZodArray.create(this);
	}
	promise() {
		return ZodPromise.create(this, this._def);
	}
	or(option) {
		return ZodUnion.create([this, option], this._def);
	}
	and(incoming) {
		return ZodIntersection.create(this, incoming, this._def);
	}
	transform(transform) {
		return new ZodEffects({
			...processCreateParams(this._def),
			schema: this,
			typeName: ZodFirstPartyTypeKind.ZodEffects,
			effect: {
				type: "transform",
				transform
			}
		});
	}
	default(def) {
		const defaultValueFunc = typeof def === "function" ? def : () => def;
		return new ZodDefault({
			...processCreateParams(this._def),
			innerType: this,
			defaultValue: defaultValueFunc,
			typeName: ZodFirstPartyTypeKind.ZodDefault
		});
	}
	brand() {
		return new ZodBranded({
			typeName: ZodFirstPartyTypeKind.ZodBranded,
			type: this,
			...processCreateParams(this._def)
		});
	}
	catch(def) {
		const catchValueFunc = typeof def === "function" ? def : () => def;
		return new ZodCatch({
			...processCreateParams(this._def),
			innerType: this,
			catchValue: catchValueFunc,
			typeName: ZodFirstPartyTypeKind.ZodCatch
		});
	}
	describe(description) {
		const This = this.constructor;
		return new This({
			...this._def,
			description
		});
	}
	pipe(target) {
		return ZodPipeline.create(this, target);
	}
	readonly() {
		return ZodReadonly.create(this);
	}
	isOptional() {
		return this.safeParse(void 0).success;
	}
	isNullable() {
		return this.safeParse(null).success;
	}
};
var cuidRegex = /^c[^\s-]{8,}$/i;
var cuid2Regex = /^[0-9a-z]+$/;
var ulidRegex = /^[0-9A-HJKMNP-TV-Z]{26}$/i;
var uuidRegex = /^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$/i;
var nanoidRegex = /^[a-z0-9_-]{21}$/i;
var jwtRegex = /^[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]*$/;
var durationRegex = /^[-+]?P(?!$)(?:(?:[-+]?\d+Y)|(?:[-+]?\d+[.,]\d+Y$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:(?:[-+]?\d+W)|(?:[-+]?\d+[.,]\d+W$))?(?:(?:[-+]?\d+D)|(?:[-+]?\d+[.,]\d+D$))?(?:T(?=[\d+-])(?:(?:[-+]?\d+H)|(?:[-+]?\d+[.,]\d+H$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:[-+]?\d+(?:[.,]\d+)?S)?)??$/;
var emailRegex = /^(?!\.)(?!.*\.\.)([A-Z0-9_'+\-\.]*)[A-Z0-9_+-]@([A-Z0-9][A-Z0-9\-]*\.)+[A-Z]{2,}$/i;
var _emojiRegex = `^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$`;
var emojiRegex;
var ipv4Regex = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/;
var ipv4CidrRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/(3[0-2]|[12]?[0-9])$/;
var ipv6Regex = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$/;
var ipv6CidrRegex = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/;
var base64Regex = /^([0-9a-zA-Z+/]{4})*(([0-9a-zA-Z+/]{2}==)|([0-9a-zA-Z+/]{3}=))?$/;
var base64urlRegex = /^([0-9a-zA-Z-_]{4})*(([0-9a-zA-Z-_]{2}(==)?)|([0-9a-zA-Z-_]{3}(=)?))?$/;
var dateRegexSource = `((\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-((0[13578]|1[02])-(0[1-9]|[12]\\d|3[01])|(0[469]|11)-(0[1-9]|[12]\\d|30)|(02)-(0[1-9]|1\\d|2[0-8])))`;
var dateRegex = new RegExp(`^${dateRegexSource}$`);
function timeRegexSource(args) {
	let secondsRegexSource = `[0-5]\\d`;
	if (args.precision) secondsRegexSource = `${secondsRegexSource}\\.\\d{${args.precision}}`;
	else if (args.precision == null) secondsRegexSource = `${secondsRegexSource}(\\.\\d+)?`;
	const secondsQuantifier = args.precision ? "+" : "?";
	return `([01]\\d|2[0-3]):[0-5]\\d(:${secondsRegexSource})${secondsQuantifier}`;
}
function timeRegex(args) {
	return new RegExp(`^${timeRegexSource(args)}$`);
}
function datetimeRegex(args) {
	let regex = `${dateRegexSource}T${timeRegexSource(args)}`;
	const opts = [];
	opts.push(args.local ? `Z?` : `Z`);
	if (args.offset) opts.push(`([+-]\\d{2}:?\\d{2})`);
	regex = `${regex}(${opts.join("|")})`;
	return new RegExp(`^${regex}$`);
}
function isValidIP(ip, version) {
	if ((version === "v4" || !version) && ipv4Regex.test(ip)) return true;
	if ((version === "v6" || !version) && ipv6Regex.test(ip)) return true;
	return false;
}
function isValidJWT(jwt, alg) {
	if (!jwtRegex.test(jwt)) return false;
	try {
		const [header] = jwt.split(".");
		if (!header) return false;
		const base64 = header.replace(/-/g, "+").replace(/_/g, "/").padEnd(header.length + (4 - header.length % 4) % 4, "=");
		const decoded = JSON.parse(atob(base64));
		if (typeof decoded !== "object" || decoded === null) return false;
		if ("typ" in decoded && decoded?.typ !== "JWT") return false;
		if (!decoded.alg) return false;
		if (alg && decoded.alg !== alg) return false;
		return true;
	} catch {
		return false;
	}
}
function isValidCidr(ip, version) {
	if ((version === "v4" || !version) && ipv4CidrRegex.test(ip)) return true;
	if ((version === "v6" || !version) && ipv6CidrRegex.test(ip)) return true;
	return false;
}
var ZodString = class ZodString extends ZodType {
	_parse(input) {
		if (this._def.coerce) input.data = String(input.data);
		if (this._getType(input) !== ZodParsedType.string) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.string,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const status = new ParseStatus();
		let ctx = void 0;
		for (const check of this._def.checks) if (check.kind === "min") {
			if (input.data.length < check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					minimum: check.value,
					type: "string",
					inclusive: true,
					exact: false,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "max") {
			if (input.data.length > check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					maximum: check.value,
					type: "string",
					inclusive: true,
					exact: false,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "length") {
			const tooBig = input.data.length > check.value;
			const tooSmall = input.data.length < check.value;
			if (tooBig || tooSmall) {
				ctx = this._getOrReturnCtx(input, ctx);
				if (tooBig) addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					maximum: check.value,
					type: "string",
					inclusive: true,
					exact: true,
					message: check.message
				});
				else if (tooSmall) addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					minimum: check.value,
					type: "string",
					inclusive: true,
					exact: true,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "email") {
			if (!emailRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "email",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "emoji") {
			if (!emojiRegex) emojiRegex = new RegExp(_emojiRegex, "u");
			if (!emojiRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "emoji",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "uuid") {
			if (!uuidRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "uuid",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "nanoid") {
			if (!nanoidRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "nanoid",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "cuid") {
			if (!cuidRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "cuid",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "cuid2") {
			if (!cuid2Regex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "cuid2",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "ulid") {
			if (!ulidRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "ulid",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "url") try {
			new URL(input.data);
		} catch {
			ctx = this._getOrReturnCtx(input, ctx);
			addIssueToContext(ctx, {
				validation: "url",
				code: ZodIssueCode.invalid_string,
				message: check.message
			});
			status.dirty();
		}
		else if (check.kind === "regex") {
			check.regex.lastIndex = 0;
			if (!check.regex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "regex",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "trim") input.data = input.data.trim();
		else if (check.kind === "includes") {
			if (!input.data.includes(check.value, check.position)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: {
						includes: check.value,
						position: check.position
					},
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "toLowerCase") input.data = input.data.toLowerCase();
		else if (check.kind === "toUpperCase") input.data = input.data.toUpperCase();
		else if (check.kind === "startsWith") {
			if (!input.data.startsWith(check.value)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: { startsWith: check.value },
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "endsWith") {
			if (!input.data.endsWith(check.value)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: { endsWith: check.value },
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "datetime") {
			if (!datetimeRegex(check).test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: "datetime",
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "date") {
			if (!dateRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: "date",
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "time") {
			if (!timeRegex(check).test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_string,
					validation: "time",
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "duration") {
			if (!durationRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "duration",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "ip") {
			if (!isValidIP(input.data, check.version)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "ip",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "jwt") {
			if (!isValidJWT(input.data, check.alg)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "jwt",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "cidr") {
			if (!isValidCidr(input.data, check.version)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "cidr",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "base64") {
			if (!base64Regex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "base64",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "base64url") {
			if (!base64urlRegex.test(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					validation: "base64url",
					code: ZodIssueCode.invalid_string,
					message: check.message
				});
				status.dirty();
			}
		} else util.assertNever(check);
		return {
			status: status.value,
			value: input.data
		};
	}
	_regex(regex, validation, message) {
		return this.refinement((data) => regex.test(data), {
			validation,
			code: ZodIssueCode.invalid_string,
			...errorUtil.errToObj(message)
		});
	}
	_addCheck(check) {
		return new ZodString({
			...this._def,
			checks: [...this._def.checks, check]
		});
	}
	email(message) {
		return this._addCheck({
			kind: "email",
			...errorUtil.errToObj(message)
		});
	}
	url(message) {
		return this._addCheck({
			kind: "url",
			...errorUtil.errToObj(message)
		});
	}
	emoji(message) {
		return this._addCheck({
			kind: "emoji",
			...errorUtil.errToObj(message)
		});
	}
	uuid(message) {
		return this._addCheck({
			kind: "uuid",
			...errorUtil.errToObj(message)
		});
	}
	nanoid(message) {
		return this._addCheck({
			kind: "nanoid",
			...errorUtil.errToObj(message)
		});
	}
	cuid(message) {
		return this._addCheck({
			kind: "cuid",
			...errorUtil.errToObj(message)
		});
	}
	cuid2(message) {
		return this._addCheck({
			kind: "cuid2",
			...errorUtil.errToObj(message)
		});
	}
	ulid(message) {
		return this._addCheck({
			kind: "ulid",
			...errorUtil.errToObj(message)
		});
	}
	base64(message) {
		return this._addCheck({
			kind: "base64",
			...errorUtil.errToObj(message)
		});
	}
	base64url(message) {
		return this._addCheck({
			kind: "base64url",
			...errorUtil.errToObj(message)
		});
	}
	jwt(options) {
		return this._addCheck({
			kind: "jwt",
			...errorUtil.errToObj(options)
		});
	}
	ip(options) {
		return this._addCheck({
			kind: "ip",
			...errorUtil.errToObj(options)
		});
	}
	cidr(options) {
		return this._addCheck({
			kind: "cidr",
			...errorUtil.errToObj(options)
		});
	}
	datetime(options) {
		if (typeof options === "string") return this._addCheck({
			kind: "datetime",
			precision: null,
			offset: false,
			local: false,
			message: options
		});
		return this._addCheck({
			kind: "datetime",
			precision: typeof options?.precision === "undefined" ? null : options?.precision,
			offset: options?.offset ?? false,
			local: options?.local ?? false,
			...errorUtil.errToObj(options?.message)
		});
	}
	date(message) {
		return this._addCheck({
			kind: "date",
			message
		});
	}
	time(options) {
		if (typeof options === "string") return this._addCheck({
			kind: "time",
			precision: null,
			message: options
		});
		return this._addCheck({
			kind: "time",
			precision: typeof options?.precision === "undefined" ? null : options?.precision,
			...errorUtil.errToObj(options?.message)
		});
	}
	duration(message) {
		return this._addCheck({
			kind: "duration",
			...errorUtil.errToObj(message)
		});
	}
	regex(regex, message) {
		return this._addCheck({
			kind: "regex",
			regex,
			...errorUtil.errToObj(message)
		});
	}
	includes(value, options) {
		return this._addCheck({
			kind: "includes",
			value,
			position: options?.position,
			...errorUtil.errToObj(options?.message)
		});
	}
	startsWith(value, message) {
		return this._addCheck({
			kind: "startsWith",
			value,
			...errorUtil.errToObj(message)
		});
	}
	endsWith(value, message) {
		return this._addCheck({
			kind: "endsWith",
			value,
			...errorUtil.errToObj(message)
		});
	}
	min(minLength, message) {
		return this._addCheck({
			kind: "min",
			value: minLength,
			...errorUtil.errToObj(message)
		});
	}
	max(maxLength, message) {
		return this._addCheck({
			kind: "max",
			value: maxLength,
			...errorUtil.errToObj(message)
		});
	}
	length(len, message) {
		return this._addCheck({
			kind: "length",
			value: len,
			...errorUtil.errToObj(message)
		});
	}
	/**
	* Equivalent to `.min(1)`
	*/
	nonempty(message) {
		return this.min(1, errorUtil.errToObj(message));
	}
	trim() {
		return new ZodString({
			...this._def,
			checks: [...this._def.checks, { kind: "trim" }]
		});
	}
	toLowerCase() {
		return new ZodString({
			...this._def,
			checks: [...this._def.checks, { kind: "toLowerCase" }]
		});
	}
	toUpperCase() {
		return new ZodString({
			...this._def,
			checks: [...this._def.checks, { kind: "toUpperCase" }]
		});
	}
	get isDatetime() {
		return !!this._def.checks.find((ch) => ch.kind === "datetime");
	}
	get isDate() {
		return !!this._def.checks.find((ch) => ch.kind === "date");
	}
	get isTime() {
		return !!this._def.checks.find((ch) => ch.kind === "time");
	}
	get isDuration() {
		return !!this._def.checks.find((ch) => ch.kind === "duration");
	}
	get isEmail() {
		return !!this._def.checks.find((ch) => ch.kind === "email");
	}
	get isURL() {
		return !!this._def.checks.find((ch) => ch.kind === "url");
	}
	get isEmoji() {
		return !!this._def.checks.find((ch) => ch.kind === "emoji");
	}
	get isUUID() {
		return !!this._def.checks.find((ch) => ch.kind === "uuid");
	}
	get isNANOID() {
		return !!this._def.checks.find((ch) => ch.kind === "nanoid");
	}
	get isCUID() {
		return !!this._def.checks.find((ch) => ch.kind === "cuid");
	}
	get isCUID2() {
		return !!this._def.checks.find((ch) => ch.kind === "cuid2");
	}
	get isULID() {
		return !!this._def.checks.find((ch) => ch.kind === "ulid");
	}
	get isIP() {
		return !!this._def.checks.find((ch) => ch.kind === "ip");
	}
	get isCIDR() {
		return !!this._def.checks.find((ch) => ch.kind === "cidr");
	}
	get isBase64() {
		return !!this._def.checks.find((ch) => ch.kind === "base64");
	}
	get isBase64url() {
		return !!this._def.checks.find((ch) => ch.kind === "base64url");
	}
	get minLength() {
		let min = null;
		for (const ch of this._def.checks) if (ch.kind === "min") {
			if (min === null || ch.value > min) min = ch.value;
		}
		return min;
	}
	get maxLength() {
		let max = null;
		for (const ch of this._def.checks) if (ch.kind === "max") {
			if (max === null || ch.value < max) max = ch.value;
		}
		return max;
	}
};
ZodString.create = (params) => {
	return new ZodString({
		checks: [],
		typeName: ZodFirstPartyTypeKind.ZodString,
		coerce: params?.coerce ?? false,
		...processCreateParams(params)
	});
};
function floatSafeRemainder(val, step) {
	const valDecCount = (val.toString().split(".")[1] || "").length;
	const stepDecCount = (step.toString().split(".")[1] || "").length;
	const decCount = valDecCount > stepDecCount ? valDecCount : stepDecCount;
	return Number.parseInt(val.toFixed(decCount).replace(".", "")) % Number.parseInt(step.toFixed(decCount).replace(".", "")) / 10 ** decCount;
}
var ZodNumber = class ZodNumber extends ZodType {
	constructor() {
		super(...arguments);
		this.min = this.gte;
		this.max = this.lte;
		this.step = this.multipleOf;
	}
	_parse(input) {
		if (this._def.coerce) input.data = Number(input.data);
		if (this._getType(input) !== ZodParsedType.number) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.number,
				received: ctx.parsedType
			});
			return INVALID;
		}
		let ctx = void 0;
		const status = new ParseStatus();
		for (const check of this._def.checks) if (check.kind === "int") {
			if (!util.isInteger(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.invalid_type,
					expected: "integer",
					received: "float",
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "min") {
			if (check.inclusive ? input.data < check.value : input.data <= check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					minimum: check.value,
					type: "number",
					inclusive: check.inclusive,
					exact: false,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "max") {
			if (check.inclusive ? input.data > check.value : input.data >= check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					maximum: check.value,
					type: "number",
					inclusive: check.inclusive,
					exact: false,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "multipleOf") {
			if (floatSafeRemainder(input.data, check.value) !== 0) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.not_multiple_of,
					multipleOf: check.value,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "finite") {
			if (!Number.isFinite(input.data)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.not_finite,
					message: check.message
				});
				status.dirty();
			}
		} else util.assertNever(check);
		return {
			status: status.value,
			value: input.data
		};
	}
	gte(value, message) {
		return this.setLimit("min", value, true, errorUtil.toString(message));
	}
	gt(value, message) {
		return this.setLimit("min", value, false, errorUtil.toString(message));
	}
	lte(value, message) {
		return this.setLimit("max", value, true, errorUtil.toString(message));
	}
	lt(value, message) {
		return this.setLimit("max", value, false, errorUtil.toString(message));
	}
	setLimit(kind, value, inclusive, message) {
		return new ZodNumber({
			...this._def,
			checks: [...this._def.checks, {
				kind,
				value,
				inclusive,
				message: errorUtil.toString(message)
			}]
		});
	}
	_addCheck(check) {
		return new ZodNumber({
			...this._def,
			checks: [...this._def.checks, check]
		});
	}
	int(message) {
		return this._addCheck({
			kind: "int",
			message: errorUtil.toString(message)
		});
	}
	positive(message) {
		return this._addCheck({
			kind: "min",
			value: 0,
			inclusive: false,
			message: errorUtil.toString(message)
		});
	}
	negative(message) {
		return this._addCheck({
			kind: "max",
			value: 0,
			inclusive: false,
			message: errorUtil.toString(message)
		});
	}
	nonpositive(message) {
		return this._addCheck({
			kind: "max",
			value: 0,
			inclusive: true,
			message: errorUtil.toString(message)
		});
	}
	nonnegative(message) {
		return this._addCheck({
			kind: "min",
			value: 0,
			inclusive: true,
			message: errorUtil.toString(message)
		});
	}
	multipleOf(value, message) {
		return this._addCheck({
			kind: "multipleOf",
			value,
			message: errorUtil.toString(message)
		});
	}
	finite(message) {
		return this._addCheck({
			kind: "finite",
			message: errorUtil.toString(message)
		});
	}
	safe(message) {
		return this._addCheck({
			kind: "min",
			inclusive: true,
			value: Number.MIN_SAFE_INTEGER,
			message: errorUtil.toString(message)
		})._addCheck({
			kind: "max",
			inclusive: true,
			value: Number.MAX_SAFE_INTEGER,
			message: errorUtil.toString(message)
		});
	}
	get minValue() {
		let min = null;
		for (const ch of this._def.checks) if (ch.kind === "min") {
			if (min === null || ch.value > min) min = ch.value;
		}
		return min;
	}
	get maxValue() {
		let max = null;
		for (const ch of this._def.checks) if (ch.kind === "max") {
			if (max === null || ch.value < max) max = ch.value;
		}
		return max;
	}
	get isInt() {
		return !!this._def.checks.find((ch) => ch.kind === "int" || ch.kind === "multipleOf" && util.isInteger(ch.value));
	}
	get isFinite() {
		let max = null;
		let min = null;
		for (const ch of this._def.checks) if (ch.kind === "finite" || ch.kind === "int" || ch.kind === "multipleOf") return true;
		else if (ch.kind === "min") {
			if (min === null || ch.value > min) min = ch.value;
		} else if (ch.kind === "max") {
			if (max === null || ch.value < max) max = ch.value;
		}
		return Number.isFinite(min) && Number.isFinite(max);
	}
};
ZodNumber.create = (params) => {
	return new ZodNumber({
		checks: [],
		typeName: ZodFirstPartyTypeKind.ZodNumber,
		coerce: params?.coerce || false,
		...processCreateParams(params)
	});
};
var ZodBigInt = class ZodBigInt extends ZodType {
	constructor() {
		super(...arguments);
		this.min = this.gte;
		this.max = this.lte;
	}
	_parse(input) {
		if (this._def.coerce) try {
			input.data = BigInt(input.data);
		} catch {
			return this._getInvalidInput(input);
		}
		if (this._getType(input) !== ZodParsedType.bigint) return this._getInvalidInput(input);
		let ctx = void 0;
		const status = new ParseStatus();
		for (const check of this._def.checks) if (check.kind === "min") {
			if (check.inclusive ? input.data < check.value : input.data <= check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					type: "bigint",
					minimum: check.value,
					inclusive: check.inclusive,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "max") {
			if (check.inclusive ? input.data > check.value : input.data >= check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					type: "bigint",
					maximum: check.value,
					inclusive: check.inclusive,
					message: check.message
				});
				status.dirty();
			}
		} else if (check.kind === "multipleOf") {
			if (input.data % check.value !== BigInt(0)) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.not_multiple_of,
					multipleOf: check.value,
					message: check.message
				});
				status.dirty();
			}
		} else util.assertNever(check);
		return {
			status: status.value,
			value: input.data
		};
	}
	_getInvalidInput(input) {
		const ctx = this._getOrReturnCtx(input);
		addIssueToContext(ctx, {
			code: ZodIssueCode.invalid_type,
			expected: ZodParsedType.bigint,
			received: ctx.parsedType
		});
		return INVALID;
	}
	gte(value, message) {
		return this.setLimit("min", value, true, errorUtil.toString(message));
	}
	gt(value, message) {
		return this.setLimit("min", value, false, errorUtil.toString(message));
	}
	lte(value, message) {
		return this.setLimit("max", value, true, errorUtil.toString(message));
	}
	lt(value, message) {
		return this.setLimit("max", value, false, errorUtil.toString(message));
	}
	setLimit(kind, value, inclusive, message) {
		return new ZodBigInt({
			...this._def,
			checks: [...this._def.checks, {
				kind,
				value,
				inclusive,
				message: errorUtil.toString(message)
			}]
		});
	}
	_addCheck(check) {
		return new ZodBigInt({
			...this._def,
			checks: [...this._def.checks, check]
		});
	}
	positive(message) {
		return this._addCheck({
			kind: "min",
			value: BigInt(0),
			inclusive: false,
			message: errorUtil.toString(message)
		});
	}
	negative(message) {
		return this._addCheck({
			kind: "max",
			value: BigInt(0),
			inclusive: false,
			message: errorUtil.toString(message)
		});
	}
	nonpositive(message) {
		return this._addCheck({
			kind: "max",
			value: BigInt(0),
			inclusive: true,
			message: errorUtil.toString(message)
		});
	}
	nonnegative(message) {
		return this._addCheck({
			kind: "min",
			value: BigInt(0),
			inclusive: true,
			message: errorUtil.toString(message)
		});
	}
	multipleOf(value, message) {
		return this._addCheck({
			kind: "multipleOf",
			value,
			message: errorUtil.toString(message)
		});
	}
	get minValue() {
		let min = null;
		for (const ch of this._def.checks) if (ch.kind === "min") {
			if (min === null || ch.value > min) min = ch.value;
		}
		return min;
	}
	get maxValue() {
		let max = null;
		for (const ch of this._def.checks) if (ch.kind === "max") {
			if (max === null || ch.value < max) max = ch.value;
		}
		return max;
	}
};
ZodBigInt.create = (params) => {
	return new ZodBigInt({
		checks: [],
		typeName: ZodFirstPartyTypeKind.ZodBigInt,
		coerce: params?.coerce ?? false,
		...processCreateParams(params)
	});
};
var ZodBoolean = class extends ZodType {
	_parse(input) {
		if (this._def.coerce) input.data = Boolean(input.data);
		if (this._getType(input) !== ZodParsedType.boolean) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.boolean,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK(input.data);
	}
};
ZodBoolean.create = (params) => {
	return new ZodBoolean({
		typeName: ZodFirstPartyTypeKind.ZodBoolean,
		coerce: params?.coerce || false,
		...processCreateParams(params)
	});
};
var ZodDate = class ZodDate extends ZodType {
	_parse(input) {
		if (this._def.coerce) input.data = new Date(input.data);
		if (this._getType(input) !== ZodParsedType.date) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.date,
				received: ctx.parsedType
			});
			return INVALID;
		}
		if (Number.isNaN(input.data.getTime())) {
			addIssueToContext(this._getOrReturnCtx(input), { code: ZodIssueCode.invalid_date });
			return INVALID;
		}
		const status = new ParseStatus();
		let ctx = void 0;
		for (const check of this._def.checks) if (check.kind === "min") {
			if (input.data.getTime() < check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					message: check.message,
					inclusive: true,
					exact: false,
					minimum: check.value,
					type: "date"
				});
				status.dirty();
			}
		} else if (check.kind === "max") {
			if (input.data.getTime() > check.value) {
				ctx = this._getOrReturnCtx(input, ctx);
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					message: check.message,
					inclusive: true,
					exact: false,
					maximum: check.value,
					type: "date"
				});
				status.dirty();
			}
		} else util.assertNever(check);
		return {
			status: status.value,
			value: new Date(input.data.getTime())
		};
	}
	_addCheck(check) {
		return new ZodDate({
			...this._def,
			checks: [...this._def.checks, check]
		});
	}
	min(minDate, message) {
		return this._addCheck({
			kind: "min",
			value: minDate.getTime(),
			message: errorUtil.toString(message)
		});
	}
	max(maxDate, message) {
		return this._addCheck({
			kind: "max",
			value: maxDate.getTime(),
			message: errorUtil.toString(message)
		});
	}
	get minDate() {
		let min = null;
		for (const ch of this._def.checks) if (ch.kind === "min") {
			if (min === null || ch.value > min) min = ch.value;
		}
		return min != null ? new Date(min) : null;
	}
	get maxDate() {
		let max = null;
		for (const ch of this._def.checks) if (ch.kind === "max") {
			if (max === null || ch.value < max) max = ch.value;
		}
		return max != null ? new Date(max) : null;
	}
};
ZodDate.create = (params) => {
	return new ZodDate({
		checks: [],
		coerce: params?.coerce || false,
		typeName: ZodFirstPartyTypeKind.ZodDate,
		...processCreateParams(params)
	});
};
var ZodSymbol = class extends ZodType {
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.symbol) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.symbol,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK(input.data);
	}
};
ZodSymbol.create = (params) => {
	return new ZodSymbol({
		typeName: ZodFirstPartyTypeKind.ZodSymbol,
		...processCreateParams(params)
	});
};
var ZodUndefined = class extends ZodType {
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.undefined) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.undefined,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK(input.data);
	}
};
ZodUndefined.create = (params) => {
	return new ZodUndefined({
		typeName: ZodFirstPartyTypeKind.ZodUndefined,
		...processCreateParams(params)
	});
};
var ZodNull = class extends ZodType {
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.null) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.null,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK(input.data);
	}
};
ZodNull.create = (params) => {
	return new ZodNull({
		typeName: ZodFirstPartyTypeKind.ZodNull,
		...processCreateParams(params)
	});
};
var ZodAny = class extends ZodType {
	constructor() {
		super(...arguments);
		this._any = true;
	}
	_parse(input) {
		return OK(input.data);
	}
};
ZodAny.create = (params) => {
	return new ZodAny({
		typeName: ZodFirstPartyTypeKind.ZodAny,
		...processCreateParams(params)
	});
};
var ZodUnknown = class extends ZodType {
	constructor() {
		super(...arguments);
		this._unknown = true;
	}
	_parse(input) {
		return OK(input.data);
	}
};
ZodUnknown.create = (params) => {
	return new ZodUnknown({
		typeName: ZodFirstPartyTypeKind.ZodUnknown,
		...processCreateParams(params)
	});
};
var ZodNever = class extends ZodType {
	_parse(input) {
		const ctx = this._getOrReturnCtx(input);
		addIssueToContext(ctx, {
			code: ZodIssueCode.invalid_type,
			expected: ZodParsedType.never,
			received: ctx.parsedType
		});
		return INVALID;
	}
};
ZodNever.create = (params) => {
	return new ZodNever({
		typeName: ZodFirstPartyTypeKind.ZodNever,
		...processCreateParams(params)
	});
};
var ZodVoid = class extends ZodType {
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.undefined) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.void,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK(input.data);
	}
};
ZodVoid.create = (params) => {
	return new ZodVoid({
		typeName: ZodFirstPartyTypeKind.ZodVoid,
		...processCreateParams(params)
	});
};
var ZodArray = class ZodArray extends ZodType {
	_parse(input) {
		const { ctx, status } = this._processInputParams(input);
		const def = this._def;
		if (ctx.parsedType !== ZodParsedType.array) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.array,
				received: ctx.parsedType
			});
			return INVALID;
		}
		if (def.exactLength !== null) {
			const tooBig = ctx.data.length > def.exactLength.value;
			const tooSmall = ctx.data.length < def.exactLength.value;
			if (tooBig || tooSmall) {
				addIssueToContext(ctx, {
					code: tooBig ? ZodIssueCode.too_big : ZodIssueCode.too_small,
					minimum: tooSmall ? def.exactLength.value : void 0,
					maximum: tooBig ? def.exactLength.value : void 0,
					type: "array",
					inclusive: true,
					exact: true,
					message: def.exactLength.message
				});
				status.dirty();
			}
		}
		if (def.minLength !== null) {
			if (ctx.data.length < def.minLength.value) {
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					minimum: def.minLength.value,
					type: "array",
					inclusive: true,
					exact: false,
					message: def.minLength.message
				});
				status.dirty();
			}
		}
		if (def.maxLength !== null) {
			if (ctx.data.length > def.maxLength.value) {
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					maximum: def.maxLength.value,
					type: "array",
					inclusive: true,
					exact: false,
					message: def.maxLength.message
				});
				status.dirty();
			}
		}
		if (ctx.common.async) return Promise.all([...ctx.data].map((item, i) => {
			return def.type._parseAsync(new ParseInputLazyPath(ctx, item, ctx.path, i));
		})).then((result) => {
			return ParseStatus.mergeArray(status, result);
		});
		const result = [...ctx.data].map((item, i) => {
			return def.type._parseSync(new ParseInputLazyPath(ctx, item, ctx.path, i));
		});
		return ParseStatus.mergeArray(status, result);
	}
	get element() {
		return this._def.type;
	}
	min(minLength, message) {
		return new ZodArray({
			...this._def,
			minLength: {
				value: minLength,
				message: errorUtil.toString(message)
			}
		});
	}
	max(maxLength, message) {
		return new ZodArray({
			...this._def,
			maxLength: {
				value: maxLength,
				message: errorUtil.toString(message)
			}
		});
	}
	length(len, message) {
		return new ZodArray({
			...this._def,
			exactLength: {
				value: len,
				message: errorUtil.toString(message)
			}
		});
	}
	nonempty(message) {
		return this.min(1, message);
	}
};
ZodArray.create = (schema, params) => {
	return new ZodArray({
		type: schema,
		minLength: null,
		maxLength: null,
		exactLength: null,
		typeName: ZodFirstPartyTypeKind.ZodArray,
		...processCreateParams(params)
	});
};
function deepPartialify(schema) {
	if (schema instanceof ZodObject) {
		const newShape = {};
		for (const key in schema.shape) {
			const fieldSchema = schema.shape[key];
			newShape[key] = ZodOptional.create(deepPartialify(fieldSchema));
		}
		return new ZodObject({
			...schema._def,
			shape: () => newShape
		});
	} else if (schema instanceof ZodArray) return new ZodArray({
		...schema._def,
		type: deepPartialify(schema.element)
	});
	else if (schema instanceof ZodOptional) return ZodOptional.create(deepPartialify(schema.unwrap()));
	else if (schema instanceof ZodNullable) return ZodNullable.create(deepPartialify(schema.unwrap()));
	else if (schema instanceof ZodTuple) return ZodTuple.create(schema.items.map((item) => deepPartialify(item)));
	else return schema;
}
var ZodObject = class ZodObject extends ZodType {
	constructor() {
		super(...arguments);
		this._cached = null;
		/**
		* @deprecated In most cases, this is no longer needed - unknown properties are now silently stripped.
		* If you want to pass through unknown properties, use `.passthrough()` instead.
		*/
		this.nonstrict = this.passthrough;
		/**
		* @deprecated Use `.extend` instead
		*  */
		this.augment = this.extend;
	}
	_getCached() {
		if (this._cached !== null) return this._cached;
		const shape = this._def.shape();
		const keys = util.objectKeys(shape);
		this._cached = {
			shape,
			keys
		};
		return this._cached;
	}
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.object) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.object,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const { status, ctx } = this._processInputParams(input);
		const { shape, keys: shapeKeys } = this._getCached();
		const extraKeys = [];
		if (!(this._def.catchall instanceof ZodNever && this._def.unknownKeys === "strip")) {
			for (const key in ctx.data) if (!shapeKeys.includes(key)) extraKeys.push(key);
		}
		const pairs = [];
		for (const key of shapeKeys) {
			const keyValidator = shape[key];
			const value = ctx.data[key];
			pairs.push({
				key: {
					status: "valid",
					value: key
				},
				value: keyValidator._parse(new ParseInputLazyPath(ctx, value, ctx.path, key)),
				alwaysSet: key in ctx.data
			});
		}
		if (this._def.catchall instanceof ZodNever) {
			const unknownKeys = this._def.unknownKeys;
			if (unknownKeys === "passthrough") for (const key of extraKeys) pairs.push({
				key: {
					status: "valid",
					value: key
				},
				value: {
					status: "valid",
					value: ctx.data[key]
				}
			});
			else if (unknownKeys === "strict") {
				if (extraKeys.length > 0) {
					addIssueToContext(ctx, {
						code: ZodIssueCode.unrecognized_keys,
						keys: extraKeys
					});
					status.dirty();
				}
			} else if (unknownKeys === "strip") {} else throw new Error(`Internal ZodObject error: invalid unknownKeys value.`);
		} else {
			const catchall = this._def.catchall;
			for (const key of extraKeys) {
				const value = ctx.data[key];
				pairs.push({
					key: {
						status: "valid",
						value: key
					},
					value: catchall._parse(new ParseInputLazyPath(ctx, value, ctx.path, key)),
					alwaysSet: key in ctx.data
				});
			}
		}
		if (ctx.common.async) return Promise.resolve().then(async () => {
			const syncPairs = [];
			for (const pair of pairs) {
				const key = await pair.key;
				const value = await pair.value;
				syncPairs.push({
					key,
					value,
					alwaysSet: pair.alwaysSet
				});
			}
			return syncPairs;
		}).then((syncPairs) => {
			return ParseStatus.mergeObjectSync(status, syncPairs);
		});
		else return ParseStatus.mergeObjectSync(status, pairs);
	}
	get shape() {
		return this._def.shape();
	}
	strict(message) {
		errorUtil.errToObj;
		return new ZodObject({
			...this._def,
			unknownKeys: "strict",
			...message !== void 0 ? { errorMap: (issue, ctx) => {
				const defaultError = this._def.errorMap?.(issue, ctx).message ?? ctx.defaultError;
				if (issue.code === "unrecognized_keys") return { message: errorUtil.errToObj(message).message ?? defaultError };
				return { message: defaultError };
			} } : {}
		});
	}
	strip() {
		return new ZodObject({
			...this._def,
			unknownKeys: "strip"
		});
	}
	passthrough() {
		return new ZodObject({
			...this._def,
			unknownKeys: "passthrough"
		});
	}
	extend(augmentation) {
		return new ZodObject({
			...this._def,
			shape: () => ({
				...this._def.shape(),
				...augmentation
			})
		});
	}
	/**
	* Prior to zod@1.0.12 there was a bug in the
	* inferred type of merged objects. Please
	* upgrade if you are experiencing issues.
	*/
	merge(merging) {
		return new ZodObject({
			unknownKeys: merging._def.unknownKeys,
			catchall: merging._def.catchall,
			shape: () => ({
				...this._def.shape(),
				...merging._def.shape()
			}),
			typeName: ZodFirstPartyTypeKind.ZodObject
		});
	}
	setKey(key, schema) {
		return this.augment({ [key]: schema });
	}
	catchall(index) {
		return new ZodObject({
			...this._def,
			catchall: index
		});
	}
	pick(mask) {
		const shape = {};
		for (const key of util.objectKeys(mask)) if (mask[key] && this.shape[key]) shape[key] = this.shape[key];
		return new ZodObject({
			...this._def,
			shape: () => shape
		});
	}
	omit(mask) {
		const shape = {};
		for (const key of util.objectKeys(this.shape)) if (!mask[key]) shape[key] = this.shape[key];
		return new ZodObject({
			...this._def,
			shape: () => shape
		});
	}
	/**
	* @deprecated
	*/
	deepPartial() {
		return deepPartialify(this);
	}
	partial(mask) {
		const newShape = {};
		for (const key of util.objectKeys(this.shape)) {
			const fieldSchema = this.shape[key];
			if (mask && !mask[key]) newShape[key] = fieldSchema;
			else newShape[key] = fieldSchema.optional();
		}
		return new ZodObject({
			...this._def,
			shape: () => newShape
		});
	}
	required(mask) {
		const newShape = {};
		for (const key of util.objectKeys(this.shape)) if (mask && !mask[key]) newShape[key] = this.shape[key];
		else {
			let newField = this.shape[key];
			while (newField instanceof ZodOptional) newField = newField._def.innerType;
			newShape[key] = newField;
		}
		return new ZodObject({
			...this._def,
			shape: () => newShape
		});
	}
	keyof() {
		return createZodEnum(util.objectKeys(this.shape));
	}
};
ZodObject.create = (shape, params) => {
	return new ZodObject({
		shape: () => shape,
		unknownKeys: "strip",
		catchall: ZodNever.create(),
		typeName: ZodFirstPartyTypeKind.ZodObject,
		...processCreateParams(params)
	});
};
ZodObject.strictCreate = (shape, params) => {
	return new ZodObject({
		shape: () => shape,
		unknownKeys: "strict",
		catchall: ZodNever.create(),
		typeName: ZodFirstPartyTypeKind.ZodObject,
		...processCreateParams(params)
	});
};
ZodObject.lazycreate = (shape, params) => {
	return new ZodObject({
		shape,
		unknownKeys: "strip",
		catchall: ZodNever.create(),
		typeName: ZodFirstPartyTypeKind.ZodObject,
		...processCreateParams(params)
	});
};
var ZodUnion = class extends ZodType {
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		const options = this._def.options;
		function handleResults(results) {
			for (const result of results) if (result.result.status === "valid") return result.result;
			for (const result of results) if (result.result.status === "dirty") {
				ctx.common.issues.push(...result.ctx.common.issues);
				return result.result;
			}
			const unionErrors = results.map((result) => new ZodError(result.ctx.common.issues));
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_union,
				unionErrors
			});
			return INVALID;
		}
		if (ctx.common.async) return Promise.all(options.map(async (option) => {
			const childCtx = {
				...ctx,
				common: {
					...ctx.common,
					issues: []
				},
				parent: null
			};
			return {
				result: await option._parseAsync({
					data: ctx.data,
					path: ctx.path,
					parent: childCtx
				}),
				ctx: childCtx
			};
		})).then(handleResults);
		else {
			let dirty = void 0;
			const issues = [];
			for (const option of options) {
				const childCtx = {
					...ctx,
					common: {
						...ctx.common,
						issues: []
					},
					parent: null
				};
				const result = option._parseSync({
					data: ctx.data,
					path: ctx.path,
					parent: childCtx
				});
				if (result.status === "valid") return result;
				else if (result.status === "dirty" && !dirty) dirty = {
					result,
					ctx: childCtx
				};
				if (childCtx.common.issues.length) issues.push(childCtx.common.issues);
			}
			if (dirty) {
				ctx.common.issues.push(...dirty.ctx.common.issues);
				return dirty.result;
			}
			const unionErrors = issues.map((issues) => new ZodError(issues));
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_union,
				unionErrors
			});
			return INVALID;
		}
	}
	get options() {
		return this._def.options;
	}
};
ZodUnion.create = (types, params) => {
	return new ZodUnion({
		options: types,
		typeName: ZodFirstPartyTypeKind.ZodUnion,
		...processCreateParams(params)
	});
};
var getDiscriminator = (type) => {
	if (type instanceof ZodLazy) return getDiscriminator(type.schema);
	else if (type instanceof ZodEffects) return getDiscriminator(type.innerType());
	else if (type instanceof ZodLiteral) return [type.value];
	else if (type instanceof ZodEnum) return type.options;
	else if (type instanceof ZodNativeEnum) return util.objectValues(type.enum);
	else if (type instanceof ZodDefault) return getDiscriminator(type._def.innerType);
	else if (type instanceof ZodUndefined) return [void 0];
	else if (type instanceof ZodNull) return [null];
	else if (type instanceof ZodOptional) return [void 0, ...getDiscriminator(type.unwrap())];
	else if (type instanceof ZodNullable) return [null, ...getDiscriminator(type.unwrap())];
	else if (type instanceof ZodBranded) return getDiscriminator(type.unwrap());
	else if (type instanceof ZodReadonly) return getDiscriminator(type.unwrap());
	else if (type instanceof ZodCatch) return getDiscriminator(type._def.innerType);
	else return [];
};
var ZodDiscriminatedUnion = class ZodDiscriminatedUnion extends ZodType {
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.object) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.object,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const discriminator = this.discriminator;
		const discriminatorValue = ctx.data[discriminator];
		const option = this.optionsMap.get(discriminatorValue);
		if (!option) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_union_discriminator,
				options: Array.from(this.optionsMap.keys()),
				path: [discriminator]
			});
			return INVALID;
		}
		if (ctx.common.async) return option._parseAsync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		});
		else return option._parseSync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		});
	}
	get discriminator() {
		return this._def.discriminator;
	}
	get options() {
		return this._def.options;
	}
	get optionsMap() {
		return this._def.optionsMap;
	}
	/**
	* The constructor of the discriminated union schema. Its behaviour is very similar to that of the normal z.union() constructor.
	* However, it only allows a union of objects, all of which need to share a discriminator property. This property must
	* have a different value for each object in the union.
	* @param discriminator the name of the discriminator property
	* @param types an array of object schemas
	* @param params
	*/
	static create(discriminator, options, params) {
		const optionsMap = /* @__PURE__ */ new Map();
		for (const type of options) {
			const discriminatorValues = getDiscriminator(type.shape[discriminator]);
			if (!discriminatorValues.length) throw new Error(`A discriminator value for key \`${discriminator}\` could not be extracted from all schema options`);
			for (const value of discriminatorValues) {
				if (optionsMap.has(value)) throw new Error(`Discriminator property ${String(discriminator)} has duplicate value ${String(value)}`);
				optionsMap.set(value, type);
			}
		}
		return new ZodDiscriminatedUnion({
			typeName: ZodFirstPartyTypeKind.ZodDiscriminatedUnion,
			discriminator,
			options,
			optionsMap,
			...processCreateParams(params)
		});
	}
};
function mergeValues(a, b) {
	const aType = getParsedType(a);
	const bType = getParsedType(b);
	if (a === b) return {
		valid: true,
		data: a
	};
	else if (aType === ZodParsedType.object && bType === ZodParsedType.object) {
		const bKeys = util.objectKeys(b);
		const sharedKeys = util.objectKeys(a).filter((key) => bKeys.indexOf(key) !== -1);
		const newObj = {
			...a,
			...b
		};
		for (const key of sharedKeys) {
			const sharedValue = mergeValues(a[key], b[key]);
			if (!sharedValue.valid) return { valid: false };
			newObj[key] = sharedValue.data;
		}
		return {
			valid: true,
			data: newObj
		};
	} else if (aType === ZodParsedType.array && bType === ZodParsedType.array) {
		if (a.length !== b.length) return { valid: false };
		const newArray = [];
		for (let index = 0; index < a.length; index++) {
			const itemA = a[index];
			const itemB = b[index];
			const sharedValue = mergeValues(itemA, itemB);
			if (!sharedValue.valid) return { valid: false };
			newArray.push(sharedValue.data);
		}
		return {
			valid: true,
			data: newArray
		};
	} else if (aType === ZodParsedType.date && bType === ZodParsedType.date && +a === +b) return {
		valid: true,
		data: a
	};
	else return { valid: false };
}
var ZodIntersection = class extends ZodType {
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		const handleParsed = (parsedLeft, parsedRight) => {
			if (isAborted(parsedLeft) || isAborted(parsedRight)) return INVALID;
			const merged = mergeValues(parsedLeft.value, parsedRight.value);
			if (!merged.valid) {
				addIssueToContext(ctx, { code: ZodIssueCode.invalid_intersection_types });
				return INVALID;
			}
			if (isDirty(parsedLeft) || isDirty(parsedRight)) status.dirty();
			return {
				status: status.value,
				value: merged.data
			};
		};
		if (ctx.common.async) return Promise.all([this._def.left._parseAsync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		}), this._def.right._parseAsync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		})]).then(([left, right]) => handleParsed(left, right));
		else return handleParsed(this._def.left._parseSync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		}), this._def.right._parseSync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		}));
	}
};
ZodIntersection.create = (left, right, params) => {
	return new ZodIntersection({
		left,
		right,
		typeName: ZodFirstPartyTypeKind.ZodIntersection,
		...processCreateParams(params)
	});
};
var ZodTuple = class ZodTuple extends ZodType {
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.array) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.array,
				received: ctx.parsedType
			});
			return INVALID;
		}
		if (ctx.data.length < this._def.items.length) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.too_small,
				minimum: this._def.items.length,
				inclusive: true,
				exact: false,
				type: "array"
			});
			return INVALID;
		}
		if (!this._def.rest && ctx.data.length > this._def.items.length) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.too_big,
				maximum: this._def.items.length,
				inclusive: true,
				exact: false,
				type: "array"
			});
			status.dirty();
		}
		const items = [...ctx.data].map((item, itemIndex) => {
			const schema = this._def.items[itemIndex] || this._def.rest;
			if (!schema) return null;
			return schema._parse(new ParseInputLazyPath(ctx, item, ctx.path, itemIndex));
		}).filter((x) => !!x);
		if (ctx.common.async) return Promise.all(items).then((results) => {
			return ParseStatus.mergeArray(status, results);
		});
		else return ParseStatus.mergeArray(status, items);
	}
	get items() {
		return this._def.items;
	}
	rest(rest) {
		return new ZodTuple({
			...this._def,
			rest
		});
	}
};
ZodTuple.create = (schemas, params) => {
	if (!Array.isArray(schemas)) throw new Error("You must pass an array of schemas to z.tuple([ ... ])");
	return new ZodTuple({
		items: schemas,
		typeName: ZodFirstPartyTypeKind.ZodTuple,
		rest: null,
		...processCreateParams(params)
	});
};
var ZodRecord = class ZodRecord extends ZodType {
	get keySchema() {
		return this._def.keyType;
	}
	get valueSchema() {
		return this._def.valueType;
	}
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.object) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.object,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const pairs = [];
		const keyType = this._def.keyType;
		const valueType = this._def.valueType;
		for (const key in ctx.data) pairs.push({
			key: keyType._parse(new ParseInputLazyPath(ctx, key, ctx.path, key)),
			value: valueType._parse(new ParseInputLazyPath(ctx, ctx.data[key], ctx.path, key)),
			alwaysSet: key in ctx.data
		});
		if (ctx.common.async) return ParseStatus.mergeObjectAsync(status, pairs);
		else return ParseStatus.mergeObjectSync(status, pairs);
	}
	get element() {
		return this._def.valueType;
	}
	static create(first, second, third) {
		if (second instanceof ZodType) return new ZodRecord({
			keyType: first,
			valueType: second,
			typeName: ZodFirstPartyTypeKind.ZodRecord,
			...processCreateParams(third)
		});
		return new ZodRecord({
			keyType: ZodString.create(),
			valueType: first,
			typeName: ZodFirstPartyTypeKind.ZodRecord,
			...processCreateParams(second)
		});
	}
};
var ZodMap = class extends ZodType {
	get keySchema() {
		return this._def.keyType;
	}
	get valueSchema() {
		return this._def.valueType;
	}
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.map) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.map,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const keyType = this._def.keyType;
		const valueType = this._def.valueType;
		const pairs = [...ctx.data.entries()].map(([key, value], index) => {
			return {
				key: keyType._parse(new ParseInputLazyPath(ctx, key, ctx.path, [index, "key"])),
				value: valueType._parse(new ParseInputLazyPath(ctx, value, ctx.path, [index, "value"]))
			};
		});
		if (ctx.common.async) {
			const finalMap = /* @__PURE__ */ new Map();
			return Promise.resolve().then(async () => {
				for (const pair of pairs) {
					const key = await pair.key;
					const value = await pair.value;
					if (key.status === "aborted" || value.status === "aborted") return INVALID;
					if (key.status === "dirty" || value.status === "dirty") status.dirty();
					finalMap.set(key.value, value.value);
				}
				return {
					status: status.value,
					value: finalMap
				};
			});
		} else {
			const finalMap = /* @__PURE__ */ new Map();
			for (const pair of pairs) {
				const key = pair.key;
				const value = pair.value;
				if (key.status === "aborted" || value.status === "aborted") return INVALID;
				if (key.status === "dirty" || value.status === "dirty") status.dirty();
				finalMap.set(key.value, value.value);
			}
			return {
				status: status.value,
				value: finalMap
			};
		}
	}
};
ZodMap.create = (keyType, valueType, params) => {
	return new ZodMap({
		valueType,
		keyType,
		typeName: ZodFirstPartyTypeKind.ZodMap,
		...processCreateParams(params)
	});
};
var ZodSet = class ZodSet extends ZodType {
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.set) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.set,
				received: ctx.parsedType
			});
			return INVALID;
		}
		const def = this._def;
		if (def.minSize !== null) {
			if (ctx.data.size < def.minSize.value) {
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_small,
					minimum: def.minSize.value,
					type: "set",
					inclusive: true,
					exact: false,
					message: def.minSize.message
				});
				status.dirty();
			}
		}
		if (def.maxSize !== null) {
			if (ctx.data.size > def.maxSize.value) {
				addIssueToContext(ctx, {
					code: ZodIssueCode.too_big,
					maximum: def.maxSize.value,
					type: "set",
					inclusive: true,
					exact: false,
					message: def.maxSize.message
				});
				status.dirty();
			}
		}
		const valueType = this._def.valueType;
		function finalizeSet(elements) {
			const parsedSet = /* @__PURE__ */ new Set();
			for (const element of elements) {
				if (element.status === "aborted") return INVALID;
				if (element.status === "dirty") status.dirty();
				parsedSet.add(element.value);
			}
			return {
				status: status.value,
				value: parsedSet
			};
		}
		const elements = [...ctx.data.values()].map((item, i) => valueType._parse(new ParseInputLazyPath(ctx, item, ctx.path, i)));
		if (ctx.common.async) return Promise.all(elements).then((elements) => finalizeSet(elements));
		else return finalizeSet(elements);
	}
	min(minSize, message) {
		return new ZodSet({
			...this._def,
			minSize: {
				value: minSize,
				message: errorUtil.toString(message)
			}
		});
	}
	max(maxSize, message) {
		return new ZodSet({
			...this._def,
			maxSize: {
				value: maxSize,
				message: errorUtil.toString(message)
			}
		});
	}
	size(size, message) {
		return this.min(size, message).max(size, message);
	}
	nonempty(message) {
		return this.min(1, message);
	}
};
ZodSet.create = (valueType, params) => {
	return new ZodSet({
		valueType,
		minSize: null,
		maxSize: null,
		typeName: ZodFirstPartyTypeKind.ZodSet,
		...processCreateParams(params)
	});
};
var ZodFunction = class ZodFunction extends ZodType {
	constructor() {
		super(...arguments);
		this.validate = this.implement;
	}
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.function) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.function,
				received: ctx.parsedType
			});
			return INVALID;
		}
		function makeArgsIssue(args, error) {
			return makeIssue({
				data: args,
				path: ctx.path,
				errorMaps: [
					ctx.common.contextualErrorMap,
					ctx.schemaErrorMap,
					getErrorMap(),
					errorMap
				].filter((x) => !!x),
				issueData: {
					code: ZodIssueCode.invalid_arguments,
					argumentsError: error
				}
			});
		}
		function makeReturnsIssue(returns, error) {
			return makeIssue({
				data: returns,
				path: ctx.path,
				errorMaps: [
					ctx.common.contextualErrorMap,
					ctx.schemaErrorMap,
					getErrorMap(),
					errorMap
				].filter((x) => !!x),
				issueData: {
					code: ZodIssueCode.invalid_return_type,
					returnTypeError: error
				}
			});
		}
		const params = { errorMap: ctx.common.contextualErrorMap };
		const fn = ctx.data;
		if (this._def.returns instanceof ZodPromise) {
			const me = this;
			return OK(async function(...args) {
				const error = new ZodError([]);
				const parsedArgs = await me._def.args.parseAsync(args, params).catch((e) => {
					error.addIssue(makeArgsIssue(args, e));
					throw error;
				});
				const result = await Reflect.apply(fn, this, parsedArgs);
				return await me._def.returns._def.type.parseAsync(result, params).catch((e) => {
					error.addIssue(makeReturnsIssue(result, e));
					throw error;
				});
			});
		} else {
			const me = this;
			return OK(function(...args) {
				const parsedArgs = me._def.args.safeParse(args, params);
				if (!parsedArgs.success) throw new ZodError([makeArgsIssue(args, parsedArgs.error)]);
				const result = Reflect.apply(fn, this, parsedArgs.data);
				const parsedReturns = me._def.returns.safeParse(result, params);
				if (!parsedReturns.success) throw new ZodError([makeReturnsIssue(result, parsedReturns.error)]);
				return parsedReturns.data;
			});
		}
	}
	parameters() {
		return this._def.args;
	}
	returnType() {
		return this._def.returns;
	}
	args(...items) {
		return new ZodFunction({
			...this._def,
			args: ZodTuple.create(items).rest(ZodUnknown.create())
		});
	}
	returns(returnType) {
		return new ZodFunction({
			...this._def,
			returns: returnType
		});
	}
	implement(func) {
		return this.parse(func);
	}
	strictImplement(func) {
		return this.parse(func);
	}
	static create(args, returns, params) {
		return new ZodFunction({
			args: args ? args : ZodTuple.create([]).rest(ZodUnknown.create()),
			returns: returns || ZodUnknown.create(),
			typeName: ZodFirstPartyTypeKind.ZodFunction,
			...processCreateParams(params)
		});
	}
};
var ZodLazy = class extends ZodType {
	get schema() {
		return this._def.getter();
	}
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		return this._def.getter()._parse({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		});
	}
};
ZodLazy.create = (getter, params) => {
	return new ZodLazy({
		getter,
		typeName: ZodFirstPartyTypeKind.ZodLazy,
		...processCreateParams(params)
	});
};
var ZodLiteral = class extends ZodType {
	_parse(input) {
		if (input.data !== this._def.value) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				received: ctx.data,
				code: ZodIssueCode.invalid_literal,
				expected: this._def.value
			});
			return INVALID;
		}
		return {
			status: "valid",
			value: input.data
		};
	}
	get value() {
		return this._def.value;
	}
};
ZodLiteral.create = (value, params) => {
	return new ZodLiteral({
		value,
		typeName: ZodFirstPartyTypeKind.ZodLiteral,
		...processCreateParams(params)
	});
};
function createZodEnum(values, params) {
	return new ZodEnum({
		values,
		typeName: ZodFirstPartyTypeKind.ZodEnum,
		...processCreateParams(params)
	});
}
var ZodEnum = class ZodEnum extends ZodType {
	_parse(input) {
		if (typeof input.data !== "string") {
			const ctx = this._getOrReturnCtx(input);
			const expectedValues = this._def.values;
			addIssueToContext(ctx, {
				expected: util.joinValues(expectedValues),
				received: ctx.parsedType,
				code: ZodIssueCode.invalid_type
			});
			return INVALID;
		}
		if (!this._cache) this._cache = new Set(this._def.values);
		if (!this._cache.has(input.data)) {
			const ctx = this._getOrReturnCtx(input);
			const expectedValues = this._def.values;
			addIssueToContext(ctx, {
				received: ctx.data,
				code: ZodIssueCode.invalid_enum_value,
				options: expectedValues
			});
			return INVALID;
		}
		return OK(input.data);
	}
	get options() {
		return this._def.values;
	}
	get enum() {
		const enumValues = {};
		for (const val of this._def.values) enumValues[val] = val;
		return enumValues;
	}
	get Values() {
		const enumValues = {};
		for (const val of this._def.values) enumValues[val] = val;
		return enumValues;
	}
	get Enum() {
		const enumValues = {};
		for (const val of this._def.values) enumValues[val] = val;
		return enumValues;
	}
	extract(values, newDef = this._def) {
		return ZodEnum.create(values, {
			...this._def,
			...newDef
		});
	}
	exclude(values, newDef = this._def) {
		return ZodEnum.create(this.options.filter((opt) => !values.includes(opt)), {
			...this._def,
			...newDef
		});
	}
};
ZodEnum.create = createZodEnum;
var ZodNativeEnum = class extends ZodType {
	_parse(input) {
		const nativeEnumValues = util.getValidEnumValues(this._def.values);
		const ctx = this._getOrReturnCtx(input);
		if (ctx.parsedType !== ZodParsedType.string && ctx.parsedType !== ZodParsedType.number) {
			const expectedValues = util.objectValues(nativeEnumValues);
			addIssueToContext(ctx, {
				expected: util.joinValues(expectedValues),
				received: ctx.parsedType,
				code: ZodIssueCode.invalid_type
			});
			return INVALID;
		}
		if (!this._cache) this._cache = new Set(util.getValidEnumValues(this._def.values));
		if (!this._cache.has(input.data)) {
			const expectedValues = util.objectValues(nativeEnumValues);
			addIssueToContext(ctx, {
				received: ctx.data,
				code: ZodIssueCode.invalid_enum_value,
				options: expectedValues
			});
			return INVALID;
		}
		return OK(input.data);
	}
	get enum() {
		return this._def.values;
	}
};
ZodNativeEnum.create = (values, params) => {
	return new ZodNativeEnum({
		values,
		typeName: ZodFirstPartyTypeKind.ZodNativeEnum,
		...processCreateParams(params)
	});
};
var ZodPromise = class extends ZodType {
	unwrap() {
		return this._def.type;
	}
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		if (ctx.parsedType !== ZodParsedType.promise && ctx.common.async === false) {
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.promise,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return OK((ctx.parsedType === ZodParsedType.promise ? ctx.data : Promise.resolve(ctx.data)).then((data) => {
			return this._def.type.parseAsync(data, {
				path: ctx.path,
				errorMap: ctx.common.contextualErrorMap
			});
		}));
	}
};
ZodPromise.create = (schema, params) => {
	return new ZodPromise({
		type: schema,
		typeName: ZodFirstPartyTypeKind.ZodPromise,
		...processCreateParams(params)
	});
};
var ZodEffects = class extends ZodType {
	innerType() {
		return this._def.schema;
	}
	sourceType() {
		return this._def.schema._def.typeName === ZodFirstPartyTypeKind.ZodEffects ? this._def.schema.sourceType() : this._def.schema;
	}
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		const effect = this._def.effect || null;
		const checkCtx = {
			addIssue: (arg) => {
				addIssueToContext(ctx, arg);
				if (arg.fatal) status.abort();
				else status.dirty();
			},
			get path() {
				return ctx.path;
			}
		};
		checkCtx.addIssue = checkCtx.addIssue.bind(checkCtx);
		if (effect.type === "preprocess") {
			const processed = effect.transform(ctx.data, checkCtx);
			if (ctx.common.async) return Promise.resolve(processed).then(async (processed) => {
				if (status.value === "aborted") return INVALID;
				const result = await this._def.schema._parseAsync({
					data: processed,
					path: ctx.path,
					parent: ctx
				});
				if (result.status === "aborted") return INVALID;
				if (result.status === "dirty") return DIRTY(result.value);
				if (status.value === "dirty") return DIRTY(result.value);
				return result;
			});
			else {
				if (status.value === "aborted") return INVALID;
				const result = this._def.schema._parseSync({
					data: processed,
					path: ctx.path,
					parent: ctx
				});
				if (result.status === "aborted") return INVALID;
				if (result.status === "dirty") return DIRTY(result.value);
				if (status.value === "dirty") return DIRTY(result.value);
				return result;
			}
		}
		if (effect.type === "refinement") {
			const executeRefinement = (acc) => {
				const result = effect.refinement(acc, checkCtx);
				if (ctx.common.async) return Promise.resolve(result);
				if (result instanceof Promise) throw new Error("Async refinement encountered during synchronous parse operation. Use .parseAsync instead.");
				return acc;
			};
			if (ctx.common.async === false) {
				const inner = this._def.schema._parseSync({
					data: ctx.data,
					path: ctx.path,
					parent: ctx
				});
				if (inner.status === "aborted") return INVALID;
				if (inner.status === "dirty") status.dirty();
				executeRefinement(inner.value);
				return {
					status: status.value,
					value: inner.value
				};
			} else return this._def.schema._parseAsync({
				data: ctx.data,
				path: ctx.path,
				parent: ctx
			}).then((inner) => {
				if (inner.status === "aborted") return INVALID;
				if (inner.status === "dirty") status.dirty();
				return executeRefinement(inner.value).then(() => {
					return {
						status: status.value,
						value: inner.value
					};
				});
			});
		}
		if (effect.type === "transform") if (ctx.common.async === false) {
			const base = this._def.schema._parseSync({
				data: ctx.data,
				path: ctx.path,
				parent: ctx
			});
			if (!isValid(base)) return INVALID;
			const result = effect.transform(base.value, checkCtx);
			if (result instanceof Promise) throw new Error(`Asynchronous transform encountered during synchronous parse operation. Use .parseAsync instead.`);
			return {
				status: status.value,
				value: result
			};
		} else return this._def.schema._parseAsync({
			data: ctx.data,
			path: ctx.path,
			parent: ctx
		}).then((base) => {
			if (!isValid(base)) return INVALID;
			return Promise.resolve(effect.transform(base.value, checkCtx)).then((result) => ({
				status: status.value,
				value: result
			}));
		});
		util.assertNever(effect);
	}
};
ZodEffects.create = (schema, effect, params) => {
	return new ZodEffects({
		schema,
		typeName: ZodFirstPartyTypeKind.ZodEffects,
		effect,
		...processCreateParams(params)
	});
};
ZodEffects.createWithPreprocess = (preprocess, schema, params) => {
	return new ZodEffects({
		schema,
		effect: {
			type: "preprocess",
			transform: preprocess
		},
		typeName: ZodFirstPartyTypeKind.ZodEffects,
		...processCreateParams(params)
	});
};
var ZodOptional = class extends ZodType {
	_parse(input) {
		if (this._getType(input) === ZodParsedType.undefined) return OK(void 0);
		return this._def.innerType._parse(input);
	}
	unwrap() {
		return this._def.innerType;
	}
};
ZodOptional.create = (type, params) => {
	return new ZodOptional({
		innerType: type,
		typeName: ZodFirstPartyTypeKind.ZodOptional,
		...processCreateParams(params)
	});
};
var ZodNullable = class extends ZodType {
	_parse(input) {
		if (this._getType(input) === ZodParsedType.null) return OK(null);
		return this._def.innerType._parse(input);
	}
	unwrap() {
		return this._def.innerType;
	}
};
ZodNullable.create = (type, params) => {
	return new ZodNullable({
		innerType: type,
		typeName: ZodFirstPartyTypeKind.ZodNullable,
		...processCreateParams(params)
	});
};
var ZodDefault = class extends ZodType {
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		let data = ctx.data;
		if (ctx.parsedType === ZodParsedType.undefined) data = this._def.defaultValue();
		return this._def.innerType._parse({
			data,
			path: ctx.path,
			parent: ctx
		});
	}
	removeDefault() {
		return this._def.innerType;
	}
};
ZodDefault.create = (type, params) => {
	return new ZodDefault({
		innerType: type,
		typeName: ZodFirstPartyTypeKind.ZodDefault,
		defaultValue: typeof params.default === "function" ? params.default : () => params.default,
		...processCreateParams(params)
	});
};
var ZodCatch = class extends ZodType {
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		const newCtx = {
			...ctx,
			common: {
				...ctx.common,
				issues: []
			}
		};
		const result = this._def.innerType._parse({
			data: newCtx.data,
			path: newCtx.path,
			parent: { ...newCtx }
		});
		if (isAsync(result)) return result.then((result) => {
			return {
				status: "valid",
				value: result.status === "valid" ? result.value : this._def.catchValue({
					get error() {
						return new ZodError(newCtx.common.issues);
					},
					input: newCtx.data
				})
			};
		});
		else return {
			status: "valid",
			value: result.status === "valid" ? result.value : this._def.catchValue({
				get error() {
					return new ZodError(newCtx.common.issues);
				},
				input: newCtx.data
			})
		};
	}
	removeCatch() {
		return this._def.innerType;
	}
};
ZodCatch.create = (type, params) => {
	return new ZodCatch({
		innerType: type,
		typeName: ZodFirstPartyTypeKind.ZodCatch,
		catchValue: typeof params.catch === "function" ? params.catch : () => params.catch,
		...processCreateParams(params)
	});
};
var ZodNaN = class extends ZodType {
	_parse(input) {
		if (this._getType(input) !== ZodParsedType.nan) {
			const ctx = this._getOrReturnCtx(input);
			addIssueToContext(ctx, {
				code: ZodIssueCode.invalid_type,
				expected: ZodParsedType.nan,
				received: ctx.parsedType
			});
			return INVALID;
		}
		return {
			status: "valid",
			value: input.data
		};
	}
};
ZodNaN.create = (params) => {
	return new ZodNaN({
		typeName: ZodFirstPartyTypeKind.ZodNaN,
		...processCreateParams(params)
	});
};
var ZodBranded = class extends ZodType {
	_parse(input) {
		const { ctx } = this._processInputParams(input);
		const data = ctx.data;
		return this._def.type._parse({
			data,
			path: ctx.path,
			parent: ctx
		});
	}
	unwrap() {
		return this._def.type;
	}
};
var ZodPipeline = class ZodPipeline extends ZodType {
	_parse(input) {
		const { status, ctx } = this._processInputParams(input);
		if (ctx.common.async) {
			const handleAsync = async () => {
				const inResult = await this._def.in._parseAsync({
					data: ctx.data,
					path: ctx.path,
					parent: ctx
				});
				if (inResult.status === "aborted") return INVALID;
				if (inResult.status === "dirty") {
					status.dirty();
					return DIRTY(inResult.value);
				} else return this._def.out._parseAsync({
					data: inResult.value,
					path: ctx.path,
					parent: ctx
				});
			};
			return handleAsync();
		} else {
			const inResult = this._def.in._parseSync({
				data: ctx.data,
				path: ctx.path,
				parent: ctx
			});
			if (inResult.status === "aborted") return INVALID;
			if (inResult.status === "dirty") {
				status.dirty();
				return {
					status: "dirty",
					value: inResult.value
				};
			} else return this._def.out._parseSync({
				data: inResult.value,
				path: ctx.path,
				parent: ctx
			});
		}
	}
	static create(a, b) {
		return new ZodPipeline({
			in: a,
			out: b,
			typeName: ZodFirstPartyTypeKind.ZodPipeline
		});
	}
};
var ZodReadonly = class extends ZodType {
	_parse(input) {
		const result = this._def.innerType._parse(input);
		const freeze = (data) => {
			if (isValid(data)) data.value = Object.freeze(data.value);
			return data;
		};
		return isAsync(result) ? result.then((data) => freeze(data)) : freeze(result);
	}
	unwrap() {
		return this._def.innerType;
	}
};
ZodReadonly.create = (type, params) => {
	return new ZodReadonly({
		innerType: type,
		typeName: ZodFirstPartyTypeKind.ZodReadonly,
		...processCreateParams(params)
	});
};
ZodObject.lazycreate;
var ZodFirstPartyTypeKind;
(function(ZodFirstPartyTypeKind) {
	ZodFirstPartyTypeKind["ZodString"] = "ZodString";
	ZodFirstPartyTypeKind["ZodNumber"] = "ZodNumber";
	ZodFirstPartyTypeKind["ZodNaN"] = "ZodNaN";
	ZodFirstPartyTypeKind["ZodBigInt"] = "ZodBigInt";
	ZodFirstPartyTypeKind["ZodBoolean"] = "ZodBoolean";
	ZodFirstPartyTypeKind["ZodDate"] = "ZodDate";
	ZodFirstPartyTypeKind["ZodSymbol"] = "ZodSymbol";
	ZodFirstPartyTypeKind["ZodUndefined"] = "ZodUndefined";
	ZodFirstPartyTypeKind["ZodNull"] = "ZodNull";
	ZodFirstPartyTypeKind["ZodAny"] = "ZodAny";
	ZodFirstPartyTypeKind["ZodUnknown"] = "ZodUnknown";
	ZodFirstPartyTypeKind["ZodNever"] = "ZodNever";
	ZodFirstPartyTypeKind["ZodVoid"] = "ZodVoid";
	ZodFirstPartyTypeKind["ZodArray"] = "ZodArray";
	ZodFirstPartyTypeKind["ZodObject"] = "ZodObject";
	ZodFirstPartyTypeKind["ZodUnion"] = "ZodUnion";
	ZodFirstPartyTypeKind["ZodDiscriminatedUnion"] = "ZodDiscriminatedUnion";
	ZodFirstPartyTypeKind["ZodIntersection"] = "ZodIntersection";
	ZodFirstPartyTypeKind["ZodTuple"] = "ZodTuple";
	ZodFirstPartyTypeKind["ZodRecord"] = "ZodRecord";
	ZodFirstPartyTypeKind["ZodMap"] = "ZodMap";
	ZodFirstPartyTypeKind["ZodSet"] = "ZodSet";
	ZodFirstPartyTypeKind["ZodFunction"] = "ZodFunction";
	ZodFirstPartyTypeKind["ZodLazy"] = "ZodLazy";
	ZodFirstPartyTypeKind["ZodLiteral"] = "ZodLiteral";
	ZodFirstPartyTypeKind["ZodEnum"] = "ZodEnum";
	ZodFirstPartyTypeKind["ZodEffects"] = "ZodEffects";
	ZodFirstPartyTypeKind["ZodNativeEnum"] = "ZodNativeEnum";
	ZodFirstPartyTypeKind["ZodOptional"] = "ZodOptional";
	ZodFirstPartyTypeKind["ZodNullable"] = "ZodNullable";
	ZodFirstPartyTypeKind["ZodDefault"] = "ZodDefault";
	ZodFirstPartyTypeKind["ZodCatch"] = "ZodCatch";
	ZodFirstPartyTypeKind["ZodPromise"] = "ZodPromise";
	ZodFirstPartyTypeKind["ZodBranded"] = "ZodBranded";
	ZodFirstPartyTypeKind["ZodPipeline"] = "ZodPipeline";
	ZodFirstPartyTypeKind["ZodReadonly"] = "ZodReadonly";
})(ZodFirstPartyTypeKind || (ZodFirstPartyTypeKind = {}));
var stringType = ZodString.create;
var numberType = ZodNumber.create;
ZodNaN.create;
ZodBigInt.create;
var booleanType = ZodBoolean.create;
ZodDate.create;
ZodSymbol.create;
ZodUndefined.create;
ZodNull.create;
ZodAny.create;
ZodUnknown.create;
ZodNever.create;
ZodVoid.create;
var arrayType = ZodArray.create;
var objectType = ZodObject.create;
ZodObject.strictCreate;
var unionType = ZodUnion.create;
ZodDiscriminatedUnion.create;
ZodIntersection.create;
var tupleType = ZodTuple.create;
ZodRecord.create;
ZodMap.create;
ZodSet.create;
ZodFunction.create;
ZodLazy.create;
var literalType = ZodLiteral.create;
ZodEnum.create;
ZodNativeEnum.create;
ZodPromise.create;
ZodEffects.create;
ZodOptional.create;
ZodNullable.create;
ZodEffects.createWithPreprocess;
ZodPipeline.create;
//#endregion
//#region src/api/schemas.ts
/**
* Runtime schemas for every response type in ../types/board.ts.
*
* The HTTP client validates each server payload against these schemas
* before it reaches the UI, so malformed JSON fails fast with an
* `ApiError` naming the offending field instead of poisoning the game
* state with `undefined`s.
*
* Field names and optionality mirror the Rust structs in
* `src/server/types.rs`: `Option<T>` serialised without
* `skip_serializing_if` → `.nullable()`, with `skip_serializing_if`
* → `.optional()`.
*/
/**
* Tie a schema to its hand-written interface in types/board.ts.
*
* Zod's `.optional()` infers `prop?: T | undefined`, which is not
* assignable to the interface's `prop?: T` under
* `exactOptionalPropertyTypes` even though both read identically. This
* cast is compile-time only (validation behaviour is untouched) and
* records the schema ↔ interface pairing in one place.
*/
function checked(schema) {
	return schema;
}
/** 0 = Black, 1 = White. */
var pieceColor = unionType([literalType(0), literalType(1)]);
/** `[row, col]`. */
var square = tupleType([numberType(), numberType()]);
var boardStateSchema = checked(objectType({
	board: arrayType(arrayType(checked(objectType({
		piece: stringType(),
		color: pieceColor,
		name: stringType(),
		value: numberType(),
		is_royal: booleanType()
	})).nullable())),
	side_to_move: pieceColor,
	move_number: numberType(),
	black_pieces: numberType(),
	white_pieces: numberType(),
	game_result: stringType().nullable(),
	version: numberType(),
	tsfen: stringType(),
	eval_score: numberType(),
	material_score: numberType(),
	no_progress_plies: numberType(),
	mode: stringType(),
	move_log: arrayType(stringType()),
	score_history: arrayType(numberType()),
	game_over: booleanType()
}));
var moveDataSchema = checked(objectType({
	from: square.nullable().optional(),
	to: square,
	promotion: booleanType(),
	is_igui: booleanType(),
	captured: stringType().nullable(),
	mid_sq: numberType().nullable().optional()
}));
/** `GET /api/moves?r=&c=` response. */
var moveListSchema = checked(objectType({
	moves: arrayType(moveDataSchema),
	from: square
}));
objectType({
	moves: arrayType(moveDataSchema),
	count: numberType(),
	side_to_move: pieceColor,
	game_over: booleanType()
});
var lastMoveSchema = checked(objectType({
	from: square,
	to: square
}));
var searchStatsSchema = checked(objectType({
	depth: numberType(),
	score: numberType(),
	nodes: numberType(),
	time_ms: numberType()
}));
var moveResponseSchema = checked(objectType({
	ok: booleanType(),
	error: stringType().nullable().optional(),
	board: boardStateSchema.nullable().optional(),
	last_move: lastMoveSchema.nullable().optional(),
	search: searchStatsSchema.nullable().optional()
}));
var undoResponseSchema = checked(objectType({
	ok: booleanType(),
	error: stringType().nullable().optional(),
	board: boardStateSchema.nullable().optional(),
	undone_plies: numberType()
}));
var newGameSchema = checked(objectType({
	ok: booleanType(),
	version: numberType()
}));
var searchResponseSchema = checked(objectType({
	ok: booleanType(),
	best_move: moveDataSchema.nullable().optional(),
	score: numberType(),
	nodes: numberType(),
	time_ms: numberType(),
	depth: numberType(),
	tsfen: stringType()
}));
objectType({
	tsfen: stringType(),
	side_to_move: pieceColor,
	move_number: numberType()
});
var saveInfoSchema = checked(objectType({
	ok: booleanType(),
	tsfen: stringType(),
	move_log: arrayType(stringType()),
	version: numberType()
}));
var pieceInfoSchema = checked(objectType({
	abbrev: stringType(),
	name: stringType(),
	value: numberType(),
	promotes_to: stringType().nullable(),
	slide_directions: numberType(),
	jump_destinations: numberType(),
	has_hook: booleanType(),
	area_steps: numberType(),
	has_range_capture: booleanType(),
	has_igui: booleanType(),
	is_royal: booleanType(),
	betza: stringType().nullable()
}));
objectType({
	count: numberType(),
	pieces: arrayType(pieceInfoSchema)
});
var engineInfoSchema = checked(objectType({
	name: stringType(),
	version: stringType(),
	board_size: numberType(),
	num_squares: numberType(),
	piece_types: numberType(),
	royal_piece_types: numberType(),
	black_pieces: numberType(),
	white_pieces: numberType(),
	side_to_move: pieceColor,
	move_number: numberType(),
	eval_backend: stringType(),
	using_nnue: booleanType(),
	hash_mb: numberType(),
	default_threads: numberType(),
	max_threads: numberType(),
	search_techniques: arrayType(stringType())
}));
objectType({
	eval_score: numberType(),
	material_score: numberType(),
	black_pieces: numberType(),
	white_pieces: numberType(),
	side_to_move: pieceColor
});
objectType({
	game_over: booleanType(),
	result: stringType().nullable(),
	half_move: numberType(),
	move_number: numberType(),
	black_pieces: numberType(),
	white_pieces: numberType(),
	move_log: arrayType(stringType()),
	tsfen: stringType(),
	version: numberType()
});
objectType({
	move_log: arrayType(stringType()),
	half_move: numberType(),
	game_over: booleanType(),
	score_history: arrayType(numberType())
});
objectType({
	ok: booleanType(),
	name: stringType(),
	version: stringType(),
	board_size: numberType(),
	piece_types: numberType()
});
//#endregion
//#region src/api/client.ts
var BASE = "";
var ApiError = class extends Error {
	status;
	constructor(message, status) {
		super(message);
		this.name = "ApiError";
		this.status = status;
	}
};
/**
* Per-tab session id sent as `X-Session-Id`. Shared-mode servers ignore it;
* a server started with `--sessions`/`TAIKYOKU_SESSIONS=1` uses it to give
* every visitor (browser tab) an isolated game. See docs/WEB.md.
*
* `sessionStorage` is deliberately used (not localStorage): two tabs of the
* same browser are two separate games. Ids are restricted to
* [A-Za-z0-9_-], the charset the server accepts.
*/
function sessionId() {
	const store = typeof sessionStorage !== "undefined" ? sessionStorage : null;
	if (!store) return "anonymous";
	let id = store.getItem("taikyoku_session");
	if (!id) {
		id = globalThis.crypto?.randomUUID?.() ?? `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
		store.setItem("taikyoku_session", id);
	}
	return id;
}
async function request(path, schema, init) {
	const res = await fetch(`${BASE}${path}`, {
		headers: {
			Accept: "application/json",
			"X-Session-Id": sessionId(),
			...init?.body ? { "Content-Type": "application/json" } : {}
		},
		...init
	});
	let payload = null;
	const text = await res.text();
	if (text) try {
		payload = JSON.parse(text);
	} catch {
		payload = null;
	}
	if (!res.ok) throw new ApiError(payload?.error ?? `HTTP ${res.status} ${res.statusText} (${path})`, res.status);
	const parsed = schema.safeParse(payload);
	if (!parsed.success) {
		const issue = parsed.error.issues[0];
		const where = issue && issue.path.length > 0 ? ` at "${issue.path.join(".")}"` : "";
		const more = parsed.error.issues.length > 1 ? ` (+${parsed.error.issues.length - 1} more)` : "";
		throw new ApiError(`Invalid response from ${path}: ${issue?.message ?? "validation failed"}${where}${more}`, res.status);
	}
	return parsed.data;
}
var get = (path, schema) => request(path, schema);
var post = (path, schema, body) => request(path, schema, body === void 0 ? { method: "POST" } : {
	method: "POST",
	body: JSON.stringify(body)
});
/** Fetch the full board state. */
function fetchState() {
	return get("/api/state", boardStateSchema);
}
/** Legal moves for the piece on `(r, c)`. */
function fetchMoves(r, c) {
	return get(`/api/moves?r=${r}&c=${c}`, moveListSchema);
}
/** Start a new game. */
function newGame() {
	return post("/api/new-game", newGameSchema);
}
/** Apply a human move. */
function sendMove(from, to, promotion = false, mid_sq) {
	return post("/api/move", moveResponseSchema, {
		from,
		to,
		promotion,
		mid_sq: mid_sq ?? null
	});
}
/** Ask the AI to search and apply a move. `depth = 0` means random. */
function sendAiMove(depth = 2, timeLimit = 3e4) {
	return post("/api/ai-move", moveResponseSchema, {
		depth,
		time_limit: timeLimit
	});
}
/** Analysis search — does not mutate the board. */
function analyze(depth = 2, timeLimit = 1e4) {
	return post("/api/search", searchResponseSchema, {
		depth,
		time_limit: timeLimit
	});
}
/** Undo the last `plies` moves (default 1). */
function undo(plies = 1) {
	return post("/api/undo", undoResponseSchema, { plies });
}
function saveGame() {
	return get("/api/save", saveInfoSchema);
}
function loadGame(tsfen) {
	return post("/api/load", boardStateSchema, { tsfen });
}
/** Piece database entry by abbreviation; throws `ApiError` 404 when unknown. */
function pieceInfo(abbrev) {
	return get(`/api/piece-info/${encodeURIComponent(abbrev)}`, pieceInfoSchema);
}
/** Engine metadata: sizes, backend, hash size, search techniques. */
function fetchEngineInfo() {
	return get("/api/engine-info", engineInfoSchema);
}
var MIN_ZOOM = .7;
var MAX_ZOOM = 1.8;
/** Map a board square to its on-screen square (flip = 180° rotation). */
function toViewSquare([r, c], flip) {
	return flip ? [35 - r, 35 - c] : [r, c];
}
/** Map an on-screen square back to the engine's board coordinates. */
function fromViewSquare([vr, vc], flip) {
	return flip ? [35 - vr, 35 - vc] : [vr, vc];
}
function clampZoom(zoom) {
	if (!Number.isFinite(zoom)) return 1;
	return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}
function sideName(color) {
	return color === 0 ? "Black" : "White";
}
/** Human-readable text for the `game_result` field. */
function describeResult(result) {
	switch (result) {
		case "black_wins": return "Black wins";
		case "white_wins": return "White wins";
		case "draw": return "Draw";
		default: return "";
	}
}
/** "+12" / "-7" / "0". */
function formatScore(score) {
	return score > 0 ? `+${score}` : String(score);
}
/** Compact node counts: 12, 4.3k, 1.2M. */
function formatNodes(nodes) {
	if (nodes >= 1e6) return `${(nodes / 1e6).toFixed(1)}M`;
	if (nodes >= 1e3) return `${(nodes / 1e3).toFixed(1)}k`;
	return String(nodes);
}
/** Convert the material score history into chart coordinates. */
function chartPoints(values, width, height, padding = 3) {
	if (values.length === 0) return [];
	const maxAbs = Math.max(1, ...values.map((v) => Math.abs(v)));
	const usableW = Math.max(1, width - padding * 2);
	const usableH = Math.max(1, height - padding * 2);
	const step = values.length > 1 ? usableW / (values.length - 1) : 0;
	return values.map((v, i) => ({
		x: padding + i * step,
		y: padding + usableH / 2 - v / maxAbs * (usableH / 2)
	}));
}
//#endregion
//#region src/game/state.ts
var AI_DELAY_MS = 120;
var sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
var GameState = class {
	board = null;
	selectedSq = null;
	legalMoves = [];
	lastMoveFrom = null;
	lastMoveTo = null;
	settings = {
		mode: "human_vs_ai",
		humanColor: 0,
		aiDepth: 2,
		useKanji: true,
		flip: false,
		zoom: 1,
		autoPlay: false
	};
	/** Engine metadata, fetched once at startup. */
	engineInfo = null;
	/** Latest static evaluation (embedded in every board snapshot). */
	evalInfo = null;
	/** Telemetry of the most recent search. */
	lastSearch = null;
	/** Piece the user clicked, with database details. */
	inspection = null;
	/** Transient user-facing message. */
	message = null;
	/** True while a network request is in flight. */
	busy = false;
	listeners = /* @__PURE__ */ new Set();
	pieceCache = /* @__PURE__ */ new Map();
	aiPending = false;
	autoLoopActive = false;
	subscribe(fn) {
		this.listeners.add(fn);
		return () => {
			this.listeners.delete(fn);
		};
	}
	notify() {
		for (const fn of this.listeners) fn();
	}
	setMessage(text, kind = "info") {
		this.message = {
			text,
			kind
		};
	}
	/** Public helper so UI actions (copy, download…) can surface feedback. */
	announce(text, kind = "info") {
		this.setMessage(text, kind);
		this.notify();
	}
	/** Enable/disable the auto-play loop and kick it when enabling. */
	setAutoPlay(on) {
		this.settings.autoPlay = on;
		this.notify();
		if (on) this.kickAutoLoop();
	}
	/** Update derived eval info from a fresh board snapshot. */
	syncEval() {
		if (!this.board) return;
		this.evalInfo = {
			eval_score: this.board.eval_score,
			material_score: this.board.material_score,
			black_pieces: this.board.black_pieces,
			white_pieces: this.board.white_pieces,
			side_to_move: this.board.side_to_move
		};
	}
	applyBoard(board) {
		this.board = board;
		this.inspection = null;
		this.syncEval();
		if (board.game_over) {
			this.settings.autoPlay = false;
			const result = board.game_result ? board.game_result.replace("_", " ") : "game over";
			this.setMessage(`Game over — ${result}`, "success");
		}
		this.notify();
	}
	/** Initialise or reload the board from the backend. */
	async init() {
		this.busy = true;
		this.notify();
		try {
			const [board, engine] = await Promise.all([fetchState(), fetchEngineInfo().catch(() => null)]);
			if (engine) this.engineInfo = engine;
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			this.applyBoard(board);
			this.setMessage("Ready", "info");
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.notify();
			throw err;
		} finally {
			this.busy = false;
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Start a new game. */
	async reset() {
		this.busy = true;
		this.notify();
		try {
			await newGame();
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			const board = await fetchState();
			this.applyBoard(board);
			this.setMessage("New game", "success");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Toggle piece symbol mode between kanji and English abbreviations. */
	toggleKanji() {
		this.settings.useKanji = !this.settings.useKanji;
		this.notify();
	}
	/** Rotate the board 180°. */
	toggleFlip() {
		this.settings.flip = !this.settings.flip;
		this.notify();
	}
	setZoom(zoom) {
		this.settings.zoom = clampZoom(zoom);
		this.notify();
	}
	/** Undo the last `plies` moves. */
	async undo(plies = 1) {
		if (!this.board) return;
		this.busy = true;
		this.notify();
		try {
			const res = await undo(plies);
			if (res.ok && res.board) {
				this.lastMoveFrom = null;
				this.lastMoveTo = null;
				this.lastSearch = null;
				this.applyBoard(res.board);
				this.setMessage(`Undid ${res.undone_plies} move(s)`, "success");
				this.settings.autoPlay = false;
			} else this.setMessage(res.error ?? "Nothing to undo", "error");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Select a square and fetch its legal moves. */
	async select(r, c) {
		if (!this.board) return;
		this.selectedSq = [r, c];
		this.notify();
		try {
			const resp = await fetchMoves(r, c);
			if (this.selectedSq?.[0] !== r || this.selectedSq?.[1] !== c) return;
			this.legalMoves = resp.moves;
		} catch (err) {
			this.legalMoves = [];
			this.setMessage(errorText(err), "error");
		}
		this.notify();
	}
	clearSelection() {
		this.selectedSq = null;
		this.legalMoves = [];
		this.notify();
	}
	/**
	* Inspect the piece on a square: shows name/value/movement data in the
	* sidebar regardless of which side is to move.
	*/
	async inspect(r, c) {
		const piece = this.board?.board[r]?.[c] ?? null;
		if (!piece) {
			this.clearInspection();
			return;
		}
		const cached = this.pieceCache.get(piece.piece) ?? null;
		this.inspection = {
			square: [r, c],
			piece,
			info: cached
		};
		this.notify();
		if (cached) return;
		try {
			const info = await pieceInfo(piece.piece);
			this.pieceCache.set(piece.piece, info);
			if (this.inspection?.piece.piece === piece.piece) this.inspection = {
				...this.inspection,
				info
			};
		} catch (err) {
			if (err instanceof ApiError && err.status === 404) {} else this.setMessage(errorText(err), "error");
		}
		this.notify();
	}
	clearInspection() {
		if (!this.inspection) return;
		this.inspection = null;
		this.notify();
	}
	/** Execute a human move. */
	async play(fr, fc, tr, tc, promotion, mid_sq) {
		if (!this.board || this.board.game_over || this.busy) return;
		this.selectedSq = null;
		this.legalMoves = [];
		try {
			const resp = await sendMove([fr, fc], [tr, tc], promotion, mid_sq);
			if (resp.ok && resp.board) {
				this.lastMoveFrom = resp.last_move?.from ?? null;
				this.lastMoveTo = resp.last_move?.to ?? null;
				this.applyBoard(resp.board);
			} else {
				this.setMessage(resp.error ?? "Illegal move", "error");
				this.notify();
			}
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Ask the AI to move once (public API used by the UI buttons). */
	async aiMove(depth, timeLimit) {
		await this.aiMoveOnce(depth, timeLimit);
		this.kickAutoLoop();
	}
	/** Run an analysis search without touching the board. */
	async analyze(depth, timeLimit = 1e4) {
		if (!this.board || this.busy) return;
		this.busy = true;
		this.notify();
		try {
			const resp = await analyze(depth ?? Math.max(1, this.settings.aiDepth), timeLimit);
			if (resp.best_move) {
				this.lastSearch = {
					depth: resp.depth,
					score: resp.score,
					nodes: resp.nodes,
					time_ms: resp.time_ms
				};
				const [fr, fc] = resp.best_move.from ?? [0, 0];
				const [tr, tc] = resp.best_move.to;
				this.setMessage(`Analysis d${resp.depth}: best ${fr},${fc}→${tr},${tc} (${resp.score})`, "info");
			} else this.setMessage("Analysis returned no move (time limit?)", "error");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Save the current position and return the TSFEN payload. */
	async save() {
		try {
			const saved = await saveGame();
			this.setMessage(`Saved position (${saved.tsfen.length} chars)`, "success");
			return saved;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			return null;
		} finally {
			this.notify();
		}
	}
	/** Load a position from TSFEN. */
	async load(tsfen) {
		this.busy = true;
		this.notify();
		try {
			const board = await loadGame(tsfen.trim());
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			this.applyBoard(board);
			this.setMessage("Position loaded", "success");
			this.settings.autoPlay = false;
			return true;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			return false;
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Effective depth: random modes always use depth 0. */
	depthForMode() {
		if (this.settings.mode === "human_vs_random" || this.settings.mode === "random_vs_random") return 0;
		return this.settings.aiDepth;
	}
	shouldAutoPlay() {
		if (!this.board || this.board.game_over || this.aiPending) return false;
		const { mode, humanColor } = this.settings;
		const side = this.board.side_to_move;
		if (mode === "ai_vs_ai" || mode === "random_vs_random") return this.settings.autoPlay;
		if (mode === "human_vs_random" || mode === "human_vs_ai") return side !== humanColor;
		return false;
	}
	async aiMoveOnce(depth, timeLimit) {
		if (!this.board || this.board.game_over || this.aiPending) return false;
		this.aiPending = true;
		this.busy = true;
		this.notify();
		try {
			const d = depth ?? this.depthForMode();
			const resp = await sendAiMove(d, timeLimit ?? (d === 0 ? 0 : 3e4));
			if (resp.ok && resp.board) {
				this.lastMoveFrom = resp.last_move?.from ?? null;
				this.lastMoveTo = resp.last_move?.to ?? null;
				this.lastSearch = resp.search ?? null;
				this.applyBoard(resp.board);
				return true;
			}
			if (resp.error) this.setMessage(resp.error, "error");
			this.settings.autoPlay = false;
			return false;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.settings.autoPlay = false;
			return false;
		} finally {
			this.aiPending = false;
			this.busy = false;
			this.notify();
		}
	}
	/** Start the auto-play loop unless one is already running. */
	kickAutoLoop() {
		this.runAutoLoop();
	}
	async runAutoLoop() {
		if (this.autoLoopActive) return;
		this.autoLoopActive = true;
		try {
			while (this.shouldAutoPlay()) {
				await sleep(AI_DELAY_MS);
				if (!await this.aiMoveOnce()) break;
			}
		} finally {
			this.autoLoopActive = false;
		}
	}
};
function errorText(err) {
	if (err instanceof Error) return err.message;
	return String(err);
}
//#endregion
//#region src/game/export.ts
/** Marker that identifies a file written by the web GUI. */
var GAME_FILE_FORMAT = "taikyokushogi-game";
/** Build the JSON-serialisable game record from a board snapshot. */
function buildGameFile(board, options = {}) {
	const exportedAt = options.exportedAt ?? /* @__PURE__ */ new Date();
	const engine = options.engine;
	return {
		format: GAME_FILE_FORMAT,
		version: 1,
		exported_at: exportedAt.toISOString(),
		app: "taikyokushogi-web",
		engine: engine ? {
			name: engine.name,
			version: engine.version
		} : null,
		tsfen: board.tsfen,
		moves: [...board.move_log],
		side: board.side_to_move === 0 ? "b" : "w",
		move_number: board.move_number,
		result: board.game_result,
		black_pieces: board.black_pieces,
		white_pieces: board.white_pieces,
		no_progress_plies: board.no_progress_plies
	};
}
/** Pretty-print a game record with a trailing newline (text-file friendly). */
function serializeGameFile(file) {
	return `${JSON.stringify(file, null, 2)}\n`;
}
/** Matches the move-log lines produced by `record_move` in the Rust server. */
var MOVE_LOG_RE = /^(\d+)\.\s+(Black|White):\s+(\S+)\s+\((\d+),(\d+)\)-\((\d+),(\d+)\)(\+)?$/;
function parseMoveLogEntry(entry) {
	const match = MOVE_LOG_RE.exec(entry.trim());
	if (!match) return null;
	return {
		side: match[2] === "Black" ? "b" : "w",
		piece: match[3],
		from: [Number(match[4]), Number(match[5])],
		to: [Number(match[6]), Number(match[7])],
		promotion: match[8] === "+"
	};
}
/** Columns 0–25 are files a–z; 26–35 are A–J (same as the CLI). */
function squareName([row, col]) {
	return `${col < 26 ? String.fromCharCode(97 + col) : String.fromCharCode(65 + col - 26)}${row + 1}`;
}
/**
* Full-move number for every ply. The engine increments `move_number` after
* White's move, so the last ply's number can be derived from the current
* snapshot and then walked back to the first logged ply.
*/
function pgnMoveNumbers(moves, board) {
	if (moves.length === 0) return [];
	let full = board.move_number - (board.side_to_move === 0 ? 1 : 0);
	for (const move of moves.slice(0, -1)) if (move.side === "w") full -= 1;
	full = Math.max(1, full);
	const numbers = [];
	for (const move of moves) {
		numbers.push(full);
		if (move.side === "w") full += 1;
	}
	return numbers;
}
function pgnMoveToken(move) {
	const suffix = move.promotion ? "+" : "";
	return `${move.piece}.${squareName(move.from)}-${squareName(move.to)}${suffix}`;
}
function wrapWords(text, width = 80) {
	const words = text.split(/\s+/).filter(Boolean);
	const lines = [];
	let line = "";
	for (const word of words) if (line.length === 0) line = word;
	else if (line.length + 1 + word.length <= width) line += ` ${word}`;
	else {
		lines.push(line);
		line = word;
	}
	if (line.length > 0) lines.push(line);
	return lines.join("\n");
}
function pgnResult(result) {
	switch (result) {
		case "black_wins": return "1-0";
		case "white_wins": return "0-1";
		case "draw": return "1/2-1/2";
		default: return "*";
	}
}
function pgnDate(when) {
	const pad = (value) => String(value).padStart(2, "0");
	return `${when.getFullYear()}.${pad(when.getMonth() + 1)}.${pad(when.getDate())}`;
}
function pgnTag(name, value) {
	return `[${name} "${value.replace(/\\/g, "\\\\").replace(/"/g, "\\\"")}"]`;
}
/** Build a PGN-style record. See the module comment for the format caveat. */
function buildPgn(board, options = {}) {
	const exportedAt = options.exportedAt ?? /* @__PURE__ */ new Date();
	const engine = options.engine;
	const moves = board.move_log.map(parseMoveLogEntry).filter((move) => move !== null);
	const result = pgnResult(board.game_result);
	const tags = [
		["Event", "Taikyoku Shogi - Web Game"],
		["Site", "Taikyoku Shogi Web GUI"],
		["Date", pgnDate(exportedAt)],
		["Round", "-"],
		["White", "?"],
		["Black", "?"],
		["Result", result],
		["Variant", "Taikyoku Shogi"],
		["BoardSize", "36x36"],
		["MoveFormat", "piece.from-to"],
		["Mode", board.mode],
		["PlyCount", String(moves.length)]
	];
	if (engine) tags.push(["Engine", `${engine.name} ${engine.version}`]);
	if (board.game_result) tags.push(["Termination", board.game_result.replace(/_/g, " ")]);
	const numbers = pgnMoveNumbers(moves, board);
	const parts = [];
	moves.forEach((move, index) => {
		const token = pgnMoveToken(move);
		if (move.side === "b" || index === 0) parts.push(`${numbers[index]}${move.side === "b" ? "." : "..."}`);
		parts.push(token);
	});
	parts.push(result);
	return `${tags.map(([name, value]) => pgnTag(name, value)).join("\n")}\n\n${wrapWords(parts.join(" "))}\n`;
}
/** Timestamped download name, e.g. `taikyoku-game-20261110-153012-move-42.json`. */
function gameFileName(when, moveNumber, format = "json") {
	const pad = (value, width = 2) => String(value).padStart(width, "0");
	return `taikyoku-game-${`${when.getFullYear()}${pad(when.getMonth() + 1)}${pad(when.getDate())}-${pad(when.getHours())}${pad(when.getMinutes())}${pad(when.getSeconds())}`}-move-${moveNumber}.${format}`;
}
//#endregion
//#region src/data/kanji.ts
/**
* Kanji symbols for all Taikyoku Shogi piece types.
*
* Sources: English names and kanji from the Wikipedia "Game equipment" section
* (https://en.wikipedia.org/wiki/Taikyoku_shogi).  The mapping key is the
* two-letter (or three-letter) abbreviation used internally by the engine.
*/
var KANJI_MAP = {
	"K": "王将",
	"CP": "太子",
	"GG": "大将",
	"G": "金将",
	"RG": "右将",
	"LG": "左将",
	"RS": "後旗",
	"Q": "奔王",
	"FT": "奔獏",
	"WO": "鳩槃",
	"CD": "鳩盤",
	"ED": "地龍",
	"FR": "奔鬼",
	"HR": "走馬",
	"BC": "獣曹",
	"LO": "天狗",
	"MR": "右山鷲",
	"ML": "左山鷲",
	"DM": "火鬼",
	"W": "鯨鯢",
	"RR": "走兎",
	"WT": "白虎",
	"TS": "玄武",
	"L": "香車",
	"RV": "反車",
	"FG": "香象",
	"WE": "白象",
	"TD": "山鳩",
	"FS": "飛燕",
	"CO": "禽吏",
	"RA": "雨龍",
	"FO": "森鬼",
	"MS": "山鹿",
	"RP": "走狗",
	"RU": "走蛇",
	"SS": "横蛇",
	"GR": "大鳩",
	"RT": "走虎",
	"BA": "走熊",
	"YA": "夜叉",
	"BD": "羅刹",
	"GU": "金剛",
	"WR": "力士",
	"S": "銀将",
	"DE": "酔象",
	"NK": "近王",
	"GC": "金車",
	"SI": "横龍",
	"RN": "走鹿",
	"RW": "走狼",
	"BG": "角将",
	"RO": "飛将",
	"TT": "右虎",
	"LT": "左虎",
	"RI": "右龍",
	"LE": "左龍",
	"BO": "獣吏",
	"WD": "風龍",
	"FP": "奔狗",
	"RB": "行鳥",
	"OK": "古鵄",
	"PC": "孔雀",
	"WA": "水龍",
	"FI": "火龍",
	"C": "銅将",
	"PM": "鳳師",
	"KM": "麟師",
	"SV": "銀車",
	"VE": "竪熊",
	"N": "桂馬",
	"PI": "豚将",
	"CG": "鶏将",
	"PG": "狗将",
	"H": "馬将",
	"O": "牛将",
	"CN": "中旗",
	"SA": "横猪",
	"SR": "銀兎",
	"GL": "金鹿",
	"LN": "獅子",
	"CT": "禽曹",
	"GS": "大鹿",
	"VD": "猛龍",
	"WL": "林鬼",
	"VG": "副将",
	"CI": "石車",
	"CE": "雲鷲",
	"B": "角行",
	"R": "飛車",
	"WF": "横狼",
	"FC": "飛猫",
	"MF": "山鷹",
	"VT": "竪虎",
	"SO": "兵士",
	"LS": "小旗",
	"CL": "雲龍",
	"CR": "銅車",
	"RH": "走車",
	"HE": "羊兵",
	"VO": "猛牛",
	"GD": "大龍",
	"GO": "金翅",
	"DS": "無明",
	"DV": "提婆",
	"WC": "木車",
	"WH": "白駒",
	"DL": "𠵇犬",
	"DR": "𠵇犬",
	"SM": "横行",
	"PR": "踊鹿",
	"WB": "水牛",
	"FL": "猛豹",
	"EG": "猛鷲",
	"FD": "飛龍",
	"PS": "毒蛇",
	"FY": "鳫飛",
	"ST": "烏行",
	"BI": "盲犬",
	"WG": "水将",
	"F": "火将",
	"PH": "鳳凰",
	"KR": "麒麟",
	"HM": "鉤行",
	"LL": "小亀",
	"GT": "大亀",
	"CA": "摩羯",
	"TC": "瓦車",
	"VW": "竪狼",
	"SX": "横牛",
	"DO": "驢馬",
	"FH": "馬麟",
	"VB": "猛熊",
	"AB": "嗔猪",
	"EW": "悪狼",
	"WI": "風馬",
	"CK": "鶏飛",
	"OM": "古猿",
	"CC": "淮鶏",
	"NB": "北狄",
	"SU": "南蛮",
	"WS": "西戎",
	"ES": "東夷",
	"VS": "猛鹿",
	"NT": "猛狼",
	"TF": "隠狐",
	"MT": "中師",
	"PE": "鵬師",
	"EC": "土車",
	"VI": "朱雀",
	"BL": "青龍",
	"EB": "変狸",
	"HO": "騎兵",
	"OW": "鴟行",
	"CM": "登猿",
	"CS": "猫刀",
	"SW": "燕羽",
	"BM": "盲猿",
	"BT": "盲虎",
	"OC": "牛車",
	"SF": "横飛",
	"BB": "盲熊",
	"OR": "老鼠",
	"SQ": "方行",
	"SN": "蟠蛇",
	"RD": "臥龍",
	"FE": "奔鷲",
	"LI": "獅鷹",
	"CH": "車兵",
	"SL": "横兵",
	"VR": "竪兵",
	"WN": "風将",
	"RE": "川将",
	"M": "山将",
	"SD": "前旗",
	"HS": "馬兵",
	"GN": "木将",
	"OS": "牛兵",
	"EA": "土将",
	"BS": "猪兵",
	"SG": "石将",
	"LP": "豹兵",
	"T": "瓦将",
	"BE": "熊兵",
	"I": "鉄将",
	"GE": "大旗",
	"GM": "大師",
	"RC": "右車",
	"LC": "左車",
	"MK": "横猿",
	"VM": "竪行",
	"OX": "飛牛",
	"LB": "弩兵",
	"VP": "竪狗",
	"VH": "竪馬",
	"BN": "炮兵",
	"DH": "龍馬",
	"DK": "龍王",
	"SE": "刀兵",
	"HF": "角鷹",
	"EL": "飛鷲",
	"SP": "鎗兵",
	"VL": "竪豹",
	"TG": "猛虎",
	"SC": "弓兵",
	"DG": "吼犬",
	"LD": "狛犬",
	"D": "犬",
	"GB": "仲人",
	"P": "歩兵"
};
/** Look up the kanji abbreviation for a piece code. */
function pieceKanji(abbrev) {
	return KANJI_MAP[abbrev] ?? abbrev;
}
//#endregion
//#region src/ui/renderer.ts
var BASE_CELL = 28;
var BASE_COORD = 18;
/** Compute the canvas geometry for a zoom factor. */
function layoutFor(zoom) {
	const cell = Math.max(10, Math.round(BASE_CELL * zoom));
	const coord = Math.max(14, Math.round(BASE_COORD * zoom));
	const boardPx = cell * 36;
	return {
		cell,
		coord,
		boardPx,
		width: coord * 2 + boardPx
	};
}
var LIGHT = "#c8b07a";
var DARK = "#a68a5b";
var BG = "#0a0a1a";
var SEL = "#ffd700";
var LEGAL_EMPTY = "rgba(92,184,92,0.55)";
var LEGAL_CAPT = "rgba(217,83,79,0.55)";
var LAST_MOVE = "rgba(74,144,217,0.45)";
var HOVER = "rgba(255,215,0,0.20)";
var INSPECT = "#4ad9d9";
var BLACK_PIECE = "#1a0505";
var WHITE_PIECE = "#f5f5ff";
var ROYAL_BLACK = "#c8a030";
var ROYAL_WHITE = "#3050c8";
var UNDERLINE = "#ccccff";
var COORD_COLOR = "#888";
function render(ctx, opts) {
	const { state, selectedSq, inspectSq, legalMoves, hoverSq, lastFrom, lastTo, useKanji, flip } = opts;
	const { cell, coord, boardPx, width } = layoutFor(opts.zoom);
	const W = width;
	const H = width;
	const cellX = (viewCol) => coord + viewCol * cell;
	const cellY = (viewRow) => coord + viewRow * cell;
	const view = (sq) => sq ? toViewSquare(sq, flip) : null;
	ctx.fillStyle = BG;
	ctx.fillRect(0, 0, W, H);
	if (!state.board) {
		ctx.fillStyle = "#999";
		ctx.font = "16px sans-serif";
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.fillText("Loading...", W / 2, H / 2);
		return;
	}
	const lastFromView = view(lastFrom);
	const lastToView = view(lastTo);
	const selectedView = view(selectedSq);
	const inspectView = view(inspectSq);
	const hoverView = view(hoverSq);
	for (let vr = 0; vr < 36; vr++) for (let vc = 0; vc < 36; vc++) {
		const x = cellX(vc);
		const y = cellY(vr);
		ctx.fillStyle = (vr + vc) % 2 === 0 ? LIGHT : DARK;
		ctx.fillRect(x, y, cell, cell);
		if (lastFromView && lastFromView[0] === vr && lastFromView[1] === vc || lastToView && lastToView[0] === vr && lastToView[1] === vc) {
			ctx.fillStyle = LAST_MOVE;
			ctx.fillRect(x, y, cell, cell);
		}
	}
	for (const m of legalMoves) {
		const [tr, tc] = m.to;
		const [vr, vc] = toViewSquare([tr, tc], flip);
		const x = cellX(vc);
		const y = cellY(vr);
		if (state.board[tr]?.[tc] != null) {
			ctx.fillStyle = LEGAL_CAPT;
			ctx.fillRect(x, y, cell, cell);
		} else {
			ctx.fillStyle = LEGAL_EMPTY;
			ctx.beginPath();
			ctx.arc(x + cell / 2, y + cell / 2, Math.max(3, cell * .18), 0, Math.PI * 2);
			ctx.fill();
		}
	}
	const symbolFont = Math.max(7, Math.round(cell * .34));
	for (let br = 0; br < 36; br++) for (let bc = 0; bc < 36; bc++) {
		const piece = state.board[br]?.[bc] ?? null;
		if (!piece) continue;
		const [vr, vc] = toViewSquare([br, bc], flip);
		const x = cellX(vc);
		const y = cellY(vr);
		if (selectedView && selectedView[0] === vr && selectedView[1] === vc) {
			ctx.fillStyle = SEL;
			ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
		}
		if (inspectView && inspectView[0] === vr && inspectView[1] === vc) {
			ctx.strokeStyle = INSPECT;
			ctx.lineWidth = 2;
			ctx.strokeRect(x + 1, y + 1, cell - 2, cell - 2);
		}
		if (hoverView && hoverView[0] === vr && hoverView[1] === vc && selectedSq === null) {
			ctx.fillStyle = HOVER;
			ctx.fillRect(x, y, cell, cell);
		}
		if (piece.is_royal) {
			ctx.fillStyle = piece.color === 0 ? ROYAL_BLACK : ROYAL_WHITE;
			ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
		}
		const symbol = useKanji ? pieceKanji(piece.piece) : piece.piece;
		ctx.font = `bold ${symbolFont}px sans-serif`;
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.fillStyle = piece.color === 0 ? BLACK_PIECE : WHITE_PIECE;
		ctx.fillText(symbol, x + cell / 2, y + cell / 2);
		if (piece.color === 1) {
			ctx.strokeStyle = UNDERLINE;
			ctx.lineWidth = 1;
			ctx.beginPath();
			ctx.moveTo(x + 5, y + cell - 4);
			ctx.lineTo(x + cell - 5, y + cell - 4);
			ctx.stroke();
		}
	}
	ctx.fillStyle = COORD_COLOR;
	ctx.font = `${Math.max(8, Math.round(9 * (cell / BASE_CELL)))}px monospace`;
	ctx.textBaseline = "middle";
	for (let i = 0; i < 36; i++) {
		const label = String(flip ? i + 1 : 36 - i);
		const x = cellX(i) + cell / 2;
		const y = cellY(i) + cell / 2;
		ctx.textAlign = "right";
		ctx.fillText(label, coord - 4, y);
		ctx.textAlign = "left";
		ctx.fillText(label, coord + boardPx + 4, y);
		ctx.textAlign = "center";
		ctx.textBaseline = "bottom";
		ctx.fillText(label, x, coord - 4);
		ctx.textBaseline = "top";
		ctx.fillText(label, x, coord + boardPx + 4);
		ctx.textBaseline = "middle";
	}
	if (state.game_over) {
		ctx.fillStyle = "rgba(10,10,26,0.55)";
		ctx.fillRect(coord, coord + boardPx / 2 - 30, boardPx, 60);
		ctx.fillStyle = "#ffd700";
		ctx.font = `bold ${Math.round(cell * .7)}px sans-serif`;
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		const label = state.game_result ? state.game_result.replace("_", " ").toUpperCase() : "GAME OVER";
		ctx.fillText(label, W / 2, coord + boardPx / 2);
	}
}
//#endregion
//#region src/ui/modal.ts
function openChoiceModal(title, description, options) {
	return new Promise((resolve) => {
		const overlay = document.createElement("div");
		overlay.className = "modal-overlay";
		const box = document.createElement("div");
		box.className = "modal-box";
		box.setAttribute("role", "dialog");
		box.setAttribute("aria-modal", "true");
		const heading = document.createElement("h2");
		heading.textContent = title;
		box.appendChild(heading);
		if (description) {
			const paragraph = document.createElement("p");
			paragraph.className = "modal-description";
			paragraph.textContent = description;
			box.appendChild(paragraph);
		}
		const list = document.createElement("div");
		list.className = "modal-options";
		const close = (value) => {
			document.removeEventListener("keydown", onKey);
			overlay.remove();
			resolve(value);
		};
		options.forEach((option, index) => {
			const button = document.createElement("button");
			button.type = "button";
			button.className = "modal-option";
			button.dataset["index"] = String(index);
			const label = document.createElement("span");
			label.className = "modal-option-label";
			label.textContent = option.label;
			button.appendChild(label);
			if (option.detail) {
				const detail = document.createElement("span");
				detail.className = "modal-option-detail";
				detail.textContent = option.detail;
				button.appendChild(detail);
			}
			button.addEventListener("click", () => close(option.value));
			list.appendChild(button);
		});
		box.appendChild(list);
		const cancel = document.createElement("button");
		cancel.type = "button";
		cancel.className = "modal-cancel";
		cancel.textContent = "Cancel (Esc)";
		cancel.addEventListener("click", () => close(null));
		box.appendChild(cancel);
		const onKey = (event) => {
			if (event.key === "Escape") {
				event.preventDefault();
				close(null);
			}
		};
		document.addEventListener("keydown", onKey);
		overlay.addEventListener("click", (event) => {
			if (event.target === overlay) close(null);
		});
		overlay.appendChild(box);
		document.body.appendChild(overlay);
		box.querySelector(".modal-option")?.focus();
	});
}
//#endregion
//#region src/ui/controller.ts
var Controller = class {
	canvas;
	ctx;
	state;
	hoverSq = null;
	rafId = 0;
	constructor(canvas, ctx, state) {
		this.canvas = canvas;
		this.ctx = ctx;
		this.state = state;
		this.bindEvents();
		state.subscribe(() => this.scheduleDraw());
	}
	/** Convert a canvas-relative pixel coordinate to a board (row, col). */
	pixelToSquare(px, py) {
		const { cell, coord } = layoutFor(this.state.settings.zoom);
		const vc = Math.floor((px - coord) / cell);
		const vr = Math.floor((py - coord) / cell);
		if (vr < 0 || vr >= 36 || vc < 0 || vc >= 36) return null;
		return fromViewSquare([vr, vc], this.state.settings.flip);
	}
	/** Get mouse position relative to the canvas, accounting for CSS scaling. */
	mousePos(e) {
		const rect = this.canvas.getBoundingClientRect();
		const sx = this.canvas.width / rect.width;
		const sy = this.canvas.height / rect.height;
		return {
			px: (e.clientX - rect.left) * sx,
			py: (e.clientY - rect.top) * sy
		};
	}
	bindEvents() {
		this.canvas.addEventListener("mousemove", (e) => this.onMouseMove(e));
		this.canvas.addEventListener("mouseleave", () => this.onMouseLeave());
		this.canvas.addEventListener("click", (e) => void this.onClick(e));
	}
	onMouseMove(e) {
		const { px, py } = this.mousePos(e);
		const sq = this.pixelToSquare(px, py);
		if (sq) {
			if (!this.hoverSq || this.hoverSq[0] !== sq[0] || this.hoverSq[1] !== sq[1]) {
				this.hoverSq = sq;
				this.canvas.style.cursor = this.cursorFor(sq);
				this.scheduleDraw();
			}
		} else if (this.hoverSq) {
			this.hoverSq = null;
			this.canvas.style.cursor = "default";
			this.scheduleDraw();
		}
	}
	cursorFor([r, c]) {
		const board = this.state.board;
		if (!board) return "default";
		if (this.state.selectedSq) {
			if (this.state.legalMoves.some((m) => m.to[0] === r && m.to[1] === c)) return "pointer";
		}
		return board.board[r]?.[c] ? "pointer" : "default";
	}
	onMouseLeave() {
		if (this.hoverSq) {
			this.hoverSq = null;
			this.scheduleDraw();
		}
	}
	async onClick(e) {
		const { px, py } = this.mousePos(e);
		const sq = this.pixelToSquare(px, py);
		if (!sq) return;
		const [r, c] = sq;
		const board = this.state.board;
		if (!board) return;
		if (this.state.selectedSq) {
			const matches = this.state.legalMoves.filter((m) => m.to[0] === r && m.to[1] === c);
			if (matches.length > 0) {
				await this.handleTarget(matches);
				return;
			}
			const clicked = board.board[r]?.[c] ?? null;
			if (clicked && clicked.color === board.side_to_move && !board.game_over) {
				this.state.clearSelection();
				await this.state.select(r, c);
				this.state.inspect(r, c);
				return;
			}
			this.state.clearSelection();
			if (clicked) this.state.inspect(r, c);
			else this.state.clearInspection();
			return;
		}
		const cell = board.board[r]?.[c] ?? null;
		if (!cell) {
			this.state.clearInspection();
			return;
		}
		if (cell.color === board.side_to_move && !board.game_over) await this.state.select(r, c);
		this.state.inspect(r, c);
	}
	/** Apply a move—asking the user only when several moves are possible. */
	async handleTarget(matches) {
		if (!this.state.board || !this.state.selectedSq) return;
		const [fr, fc] = this.state.selectedSq;
		let target = matches[0] ?? null;
		if (matches.length > 1) {
			const choices = matches.map((m, i) => ({
				label: describeMove(m, i),
				detail: [
					m.promotion ? "promotes" : null,
					m.is_igui ? "igui (capture in place)" : null,
					m.captured ? `captures ${m.captured}` : null,
					m.mid_sq != null ? `mid_sq=${m.mid_sq}` : null
				].filter(Boolean).join(" · "),
				value: m
			}));
			target = await openChoiceModal("Choose a move", `${matches.length} legal moves reach this square.`, choices);
		}
		if (!target) return;
		await this.state.play(fr, fc, target.to[0], target.to[1], target.promotion, target.mid_sq ?? null);
	}
	scheduleDraw() {
		if (this.rafId) return;
		this.rafId = requestAnimationFrame(() => {
			this.rafId = 0;
			this.draw();
		});
	}
	draw() {
		const board = this.state.board;
		if (!board) return;
		const { width } = layoutFor(this.state.settings.zoom);
		if (this.canvas.width !== width || this.canvas.height !== width) {
			this.canvas.width = width;
			this.canvas.height = width;
		}
		render(this.ctx, {
			state: board,
			selectedSq: this.state.selectedSq,
			inspectSq: this.state.inspection?.square ?? null,
			legalMoves: this.state.legalMoves,
			hoverSq: this.hoverSq,
			lastFrom: this.state.lastMoveFrom,
			lastTo: this.state.lastMoveTo,
			useKanji: this.state.settings.useKanji,
			flip: this.state.settings.flip,
			zoom: this.state.settings.zoom
		});
	}
};
function describeMove(m, index) {
	const dest = `(${m.to[0]}, ${m.to[1]})`;
	const suffix = m.promotion ? " +" : "";
	return `${index + 1}. Move to ${dest}${suffix}`;
}
//#endregion
//#region src/ui/panel.ts
var $$1 = (id) => document.getElementById(id);
function setText(id, text) {
	const el = $$1(id);
	if (el && el.textContent !== text) el.textContent = text;
}
function setDisabled(id, disabled) {
	const el = $$1(id);
	if (el) el.disabled = disabled;
}
var lastLogSignature = "";
function updatePanels(state) {
	const board = state.board;
	if (!board) return;
	setText("info-turn", sideName(board.side_to_move));
	setText("info-move", String(board.move_number));
	setText("info-black", String(board.black_pieces));
	setText("info-white", String(board.white_pieces));
	setText("info-score", formatScore(board.material_score));
	setText("info-eval", formatScore(board.eval_score));
	setText("info-progress", `${board.no_progress_plies} / 1000`);
	const statusMsg = $$1("status-msg");
	if (statusMsg) {
		const fallback = board.game_result ? describeResult(board.game_result) : board.game_over ? "Game over" : "—";
		const message = state.message?.text ?? fallback;
		const kind = state.message?.kind ?? (board.game_over ? "success" : "info");
		statusMsg.textContent = message;
		statusMsg.className = `status-msg ${kind}`;
	}
	setText("status", `${sideName(board.side_to_move)} to move · ${board.black_pieces} vs ${board.white_pieces} pieces · move ${board.move_number}`);
	setText("busy", state.busy ? "● thinking…" : "");
	const search = state.lastSearch;
	setText("search-depth", search ? `d${search.depth}` : "—");
	setText("search-score", search ? formatScore(search.score) : "—");
	setText("search-nodes", search ? formatNodes(search.nodes) : "—");
	setText("search-time", search ? `${search.time_ms} ms` : "—");
	const engine = state.engineInfo;
	setText("engine-version", engine ? `v${engine.version}` : "—");
	setText("engine-backend", engine ? engine.eval_backend : "—");
	setText("engine-hash", engine ? `${engine.hash_mb} MB` : "—");
	setText("engine-pieces", engine ? `${engine.piece_types} types` : "—");
	setText("engine-techniques", engine ? `${engine.search_techniques.length} enabled` : "—");
	updateInspector(state);
	const signature = `${board.move_log.length}:${board.move_log.at(-1) ?? ""}`;
	if (signature !== lastLogSignature) {
		lastLogSignature = signature;
		const log = $$1("move-log");
		if (log) {
			const fragment = document.createDocumentFragment();
			for (const entry of board.move_log) {
				const row = document.createElement("div");
				row.className = "entry";
				row.textContent = entry;
				fragment.appendChild(row);
			}
			if (board.move_log.length === 0) {
				const empty = document.createElement("div");
				empty.className = "entry empty";
				empty.textContent = "No moves yet";
				fragment.appendChild(empty);
			}
			log.replaceChildren(fragment);
			log.scrollTop = log.scrollHeight;
		}
	}
	drawScoreChart(board.score_history);
	const autoBtn = $$1("btn-auto");
	if (autoBtn) {
		autoBtn.textContent = state.settings.autoPlay ? "Stop" : "Auto";
		autoBtn.classList.toggle("active", state.settings.autoPlay);
	}
	const kanjiBtn = $$1("btn-kanji");
	if (kanjiBtn) kanjiBtn.textContent = state.settings.useKanji ? "Kanji" : "English";
	const flipBtn = $$1("btn-flip");
	if (flipBtn) flipBtn.textContent = state.settings.flip ? "Flip ↺" : "Flip";
	setDisabled("btn-undo", board.move_log.length === 0);
	setDisabled("btn-analyze", state.busy);
	setDisabled("btn-ai-move", state.busy || board.game_over);
	setDisabled("btn-load", state.busy);
	const tsfen = $$1("tsfen-box");
	if (tsfen && document.activeElement !== tsfen) tsfen.value = board.tsfen;
}
function updateInspector(state) {
	const inspection = state.inspection;
	const hint = $$1("inspect-hint");
	if (!inspection) {
		setText("inspect-title", "—");
		setText("inspect-name", "Click a piece");
		setText("inspect-abbrev", "");
		setText("inspect-value", "");
		setText("inspect-promotes", "");
		setText("inspect-betza", "");
		setText("inspect-tags", "");
		if (hint) hint.textContent = "Hover/click any piece to inspect its movement data.";
		return;
	}
	const { piece, info, square } = inspection;
	setText("inspect-title", `(${square[0]}, ${square[1]}) · ${sideName(piece.color)}`);
	setText("inspect-name", info?.name ?? piece.name);
	setText("inspect-abbrev", `[${piece.piece}]`);
	setText("inspect-value", `value ${info?.value ?? piece.value}`);
	setText("inspect-promotes", info?.promotes_to ? `promotes to ${info.promotes_to}` : "no promotion");
	setText("inspect-betza", info?.betza ? `Betza: ${info.betza}` : "");
	const tags = [];
	if (piece.is_royal || info?.is_royal) tags.push("royal");
	if (info) {
		if (info.slide_directions > 0) tags.push(`${info.slide_directions} slides`);
		if (info.jump_destinations > 0) tags.push(`${info.jump_destinations} jumps`);
		if (info.area_steps > 0) tags.push(`area ×${info.area_steps}`);
		if (info.has_hook) tags.push("hook");
		if (info.has_range_capture) tags.push("range capture");
		if (info.has_igui) tags.push("igui");
	}
	setText("inspect-tags", tags.join(" · "));
	if (hint) hint.textContent = info ? "" : "Loading movement data…";
}
function drawScoreChart(history) {
	const canvas = $$1("score-chart");
	if (!canvas) return;
	const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
	const cssWidth = canvas.clientWidth || 220;
	const cssHeight = canvas.clientHeight || 64;
	if (canvas.width !== Math.round(cssWidth * dpr) || canvas.height !== Math.round(cssHeight * dpr)) {
		canvas.width = Math.round(cssWidth * dpr);
		canvas.height = Math.round(cssHeight * dpr);
	}
	const ctx = canvas.getContext("2d");
	if (!ctx) return;
	ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
	ctx.clearRect(0, 0, cssWidth, cssHeight);
	ctx.strokeStyle = "rgba(255,255,255,0.15)";
	ctx.lineWidth = 1;
	ctx.beginPath();
	ctx.moveTo(0, cssHeight / 2);
	ctx.lineTo(cssWidth, cssHeight / 2);
	ctx.stroke();
	if (history.length === 0) return;
	const points = chartPoints(history, cssWidth, cssHeight, 4);
	ctx.strokeStyle = "#4a90d9";
	ctx.lineWidth = 1.5;
	ctx.beginPath();
	points.forEach((p, i) => i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y));
	ctx.stroke();
	const last = points[points.length - 1];
	if (!last) return;
	ctx.fillStyle = (history[history.length - 1] ?? 0) >= 0 ? "#5cb85c" : "#d9534f";
	ctx.beginPath();
	ctx.arc(last.x, last.y, 2.5, 0, Math.PI * 2);
	ctx.fill();
}
//#endregion
//#region src/main.ts
/**
* Application entry point.
*
* Wires the game state, canvas renderer, input controller and DOM panels
* together, then boots the first game. All UI actions are also available as
* keyboard shortcuts:
*
*   N = new game · U = undo · A = auto-play · K = kanji/English
*   F = flip board · +/- = zoom · Space = ask the AI for a move
*/
var canvas = document.getElementById("board-canvas");
var ctx = canvas.getContext("2d");
if (!ctx) throw new Error("2D canvas context is not available in this browser");
var state = new GameState();
new Controller(canvas, ctx, state);
state.subscribe(() => updatePanels(state));
var $ = (id) => document.getElementById(id);
var select = (id) => $(id);
/** Start a browser download for generated text (TSFEN, JSON, PGN…). */
function downloadText(contents, filename, type) {
	const blob = new Blob([contents], { type });
	const url = URL.createObjectURL(blob);
	const link = document.createElement("a");
	link.href = url;
	link.download = filename;
	link.click();
	URL.revokeObjectURL(url);
}
function readSettingsFromControls() {
	state.settings.mode = select("mode-select").value;
	state.settings.humanColor = Number(select("color-select").value);
	state.settings.aiDepth = Number(select("ai-depth").value);
}
function syncControlsFromSettings() {
	select("mode-select").value = state.settings.mode;
	select("color-select").value = String(state.settings.humanColor);
	select("ai-depth").value = String(state.settings.aiDepth);
	const zoom = $("zoom-range");
	zoom.value = String(Math.round(state.settings.zoom * 100));
	$("zoom-label").textContent = `${Math.round(state.settings.zoom * 100)}%`;
}
$("btn-new").addEventListener("click", () => {
	readSettingsFromControls();
	state.setAutoPlay(false);
	state.reset();
});
$("btn-undo").addEventListener("click", () => void state.undo(1));
$("btn-auto").addEventListener("click", () => {
	readSettingsFromControls();
	state.setAutoPlay(!state.settings.autoPlay);
});
$("btn-kanji").addEventListener("click", () => state.toggleKanji());
$("btn-flip").addEventListener("click", () => state.toggleFlip());
$("btn-analyze").addEventListener("click", () => void state.analyze());
$("btn-ai-move").addEventListener("click", () => void state.aiMove());
$("zoom-range").addEventListener("input", (event) => {
	const value = Number(event.target.value) / 100;
	state.setZoom(value);
	$("zoom-label").textContent = `${Math.round(state.settings.zoom * 100)}%`;
});
select("mode-select").addEventListener("change", readSettingsFromControls);
select("color-select").addEventListener("change", readSettingsFromControls);
select("ai-depth").addEventListener("change", readSettingsFromControls);
var tsfenBox = $("tsfen-box");
$("btn-save").addEventListener("click", async () => {
	const saved = await state.save();
	if (saved && document.activeElement !== tsfenBox) tsfenBox.value = saved.tsfen;
});
$("btn-load").addEventListener("click", async () => {
	readSettingsFromControls();
	const tsfen = tsfenBox.value.trim();
	if (!tsfen) {
		state.announce("Paste a TSFEN string first", "error");
		return;
	}
	state.setAutoPlay(false);
	await state.load(tsfen);
});
$("btn-copy").addEventListener("click", async () => {
	try {
		await navigator.clipboard.writeText(tsfenBox.value);
		state.announce("TSFEN copied to clipboard", "success");
	} catch {
		tsfenBox.select();
		state.announce("Clipboard blocked — text selected, press Ctrl+C", "error");
	}
});
$("btn-download").addEventListener("click", () => {
	const filename = `taikyoku-move-${state.board?.move_number ?? 1}.tsfen`;
	downloadText(tsfenBox.value, filename, "text/plain");
	state.announce("TSFEN downloaded", "success");
});
function exportGame(format) {
	const board = state.board;
	if (!board) {
		state.announce("No game to export yet", "error");
		return;
	}
	const now = /* @__PURE__ */ new Date();
	const options = {
		exportedAt: now,
		engine: state.engineInfo
	};
	const contents = format === "json" ? serializeGameFile(buildGameFile(board, options)) : buildPgn(board, options);
	const type = format === "json" ? "application/json" : "application/x-chess-pgn";
	downloadText(contents, gameFileName(now, board.move_number, format), type);
	const plies = board.move_log.length;
	state.announce(`Game exported as ${format.toUpperCase()} (${plies} ${plies === 1 ? "ply" : "plies"})`, "success");
}
$("btn-export-game").addEventListener("click", () => exportGame("json"));
$("btn-export-game-pgn").addEventListener("click", () => exportGame("pgn"));
document.addEventListener("keydown", (event) => {
	const target = event.target;
	if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")) return;
	if (event.ctrlKey || event.metaKey || event.altKey) return;
	switch (event.key.toLowerCase()) {
		case "n":
			readSettingsFromControls();
			state.setAutoPlay(false);
			state.reset();
			break;
		case "u":
			state.undo(1);
			break;
		case "a":
			readSettingsFromControls();
			state.setAutoPlay(!state.settings.autoPlay);
			break;
		case "k":
			state.toggleKanji();
			break;
		case "f":
			state.toggleFlip();
			break;
		case " ":
			event.preventDefault();
			state.aiMove();
			break;
		case "+":
		case "=":
			state.setZoom(Math.min(MAX_ZOOM, state.settings.zoom + .1));
			syncControlsFromSettings();
			break;
		case "-":
			state.setZoom(Math.max(MIN_ZOOM, state.settings.zoom - .1));
			syncControlsFromSettings();
			break;
		default: break;
	}
});
syncControlsFromSettings();
state.init().catch((err) => {
	const status = document.getElementById("status");
	if (status) status.textContent = `⚠️ Error: ${err instanceof Error ? err.message : String(err)} — is the Rust backend running on :8000?`;
	console.error("Failed to initialize game:", err);
});
//#endregion
