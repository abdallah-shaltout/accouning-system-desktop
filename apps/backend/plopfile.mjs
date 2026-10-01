const toKebab = (s) =>
    s
        ?.trim()
        .replace(/[^\w\s-]/g, "")
        .replace(/\s+/g, "-")
        .replace(/_/g, "-")
        .replace(/-+/g, "-");

const toCamel = (s) => toKebab(s).replace(/-([a-z0-9])/g, (_, c) => c.toUpperCase());
const toPascal = (s) => {
    const c = toCamel(s);
    return c.charAt(0).toUpperCase() + c.slice(1);
};

/**
 * Ported from references/accouning-system/POS-Fares-server/plopfile.mjs. Kept: the `module` and
 * `multipe-modules` generators, the prompt flow, the tsconfig-alias-patching logic (adapted to this
 * project's `apps/backend/tsconfig.json` shape — see below) and the `capitalize`/`kebab`/`pascal`
 * helpers. Dropped: the reference's `app` and `report` generators — this project has no
 * `src/apps/<app>/domains/` or `src/apps/<app>/reports/` scaffolding need yet, per the Phase A task.
 *
 * Template changes (this project uses Drizzle + Zod, not Mongoose + express-validator — see
 * .dev/modules/*.hbs): the schema template emits a Drizzle `pgTable` (id/name/createdAt/updatedAt/
 * deletedAt, `$inferSelect`/`$inferInsert` types) matching `src/domains/organization/organization.schema.ts`;
 * the service template extends `BaseService` from `@@shared/core/service.core` with a soft-delete
 * `deletedAtColumn`, matching `src/domains/adminActivity/adminActivity.service.ts`'s style; the
 * controller template extends `BaseController` from `@@shared/core/controller.core`; the route
 * template wires standard REST verbs to Zod validation middleware arrays; the validation template
 * uses `validate()` from `@@shared/middleware/validator/validation.core` plus the shared field
 * helpers in `@@shared/middleware/validator/commonValidator` (`uuidParam`, `paginationQuery`,
 * `textField`) instead of the reference's `express-validator` + `customValidator` (which the
 * reference's own template stale-imports from `@@shared/middleware/validator/customValidator` — a
 * module that doesn't exist even there; this rewrite avoids reproducing that dangling import).
 */
export default function (
    /**
     * @type {import('plop').NodePlopAPI}
     */
    plop,
) {
    plop.setGenerator("module", {
        description: "Create a new module (Drizzle schema + service + controller + route + Zod validation)",
        prompts: [
            {
                type: "input",
                name: "name",
                message: "Module Name",
            },
        ],
        actions: [
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.schema.ts",
                templateFile: ".dev/modules/module.schema.hbs",
            },
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.service.ts",
                templateFile: ".dev/modules/module.service.hbs",
            },
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.validation.rules.ts",
                templateFile: ".dev/modules/module.validation.rules.hbs",
            },
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.controller.ts",
                templateFile: ".dev/modules/module.controller.hbs",
            },
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.route.ts",
                templateFile: ".dev/modules/module.route.hbs",
            },
            {
                type: "add",
                path: "src/domains/{{name}}/{{name}}.validation.ts",
                templateFile: ".dev/modules/module.validation.hbs",
            },
            {
                type: "modify",
                path: "tsconfig.json",
                transform: (content, data) => {
                    const tsconfig = JSON.parse(content);
                    const aliasKey = `@@${data.name}/*`;
                    const aliasValue = [`./src/domains/${data.name}/*`];

                    if (!tsconfig.compilerOptions.paths[aliasKey]) {
                        tsconfig.compilerOptions.paths[aliasKey] = aliasValue;
                    }

                    // This project pins @/* (apps) and ~/* (src root) at the end of `paths`, after
                    // every alphabetically-sorted @@<domain>/* and @@shared|config/* entry — match
                    // that exact layout (see tsconfig.json's existing block) instead of the
                    // reference's ["@/*", "~/*"] tail set.
                    const tailKeys = ["@/*", "~/*"];
                    const tail = {};
                    for (const key of tailKeys) {
                        if (tsconfig.compilerOptions.paths[key]) {
                            tail[key] = tsconfig.compilerOptions.paths[key];
                            delete tsconfig.compilerOptions.paths[key];
                        }
                    }

                    const sortedEntries = Object.entries(tsconfig.compilerOptions.paths).sort(
                        ([a], [b]) => a.localeCompare(b),
                    );
                    const sortedPaths = Object.fromEntries(sortedEntries);
                    Object.assign(sortedPaths, tail);
                    tsconfig.compilerOptions.paths = sortedPaths;

                    return JSON.stringify(tsconfig, null, 4) + "\n";
                },
            },
        ],
    });

    plop.setGenerator("multipe-modules", {
        description: "Generate multiple modules (comma/space/newline separated)",
        prompts: [
            {
                type: "input",
                name: "names",
                message: "Module names (comma/space/newline separated):",
                validate: (v) => (v?.trim()?.length ? true : "Enter at least one name"),
            },
        ],
        actions: (data) => {
            const raw = (data?.names || "").trim();
            const modules = Array.from(
                new Set(
                    raw
                        .split(/[,\n\r\t ]+/)
                        .map((s) => toKebab(s))
                        .filter(Boolean),
                ),
            );
            if (!modules.length) throw new Error("No valid module names provided.");

            const actions = [];

            for (const name of modules) {
                actions.push(
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.schema.ts",
                        templateFile: ".dev/modules/module.schema.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.service.ts",
                        templateFile: ".dev/modules/module.service.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.validation.rules.ts",
                        templateFile: ".dev/modules/module.validation.rules.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.controller.ts",
                        templateFile: ".dev/modules/module.controller.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.route.ts",
                        templateFile: ".dev/modules/module.route.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                    {
                        type: "add",
                        path: "src/domains/{{name}}/{{name}}.validation.ts",
                        templateFile: ".dev/modules/module.validation.hbs",
                        data: { name },
                        abortOnFail: true,
                    },
                );
            }

            actions.push({
                type: "modify",
                path: "tsconfig.json",
                transform: (content) => {
                    const tsconfig = JSON.parse(content || "{}");
                    tsconfig.compilerOptions = tsconfig.compilerOptions || {};
                    tsconfig.compilerOptions.paths = tsconfig.compilerOptions.paths || {};

                    const tailKeys = ["@/*", "~/*"];

                    for (const name of modules) {
                        const aliasKey = `@@${name}/*`;
                        const aliasValue = [`./src/domains/${name}/*`];
                        if (!tsconfig.compilerOptions.paths[aliasKey]) {
                            tsconfig.compilerOptions.paths[aliasKey] = aliasValue;
                        }
                    }

                    const tail = {};
                    for (const key of tailKeys) {
                        if (tsconfig.compilerOptions.paths[key]) {
                            tail[key] = tsconfig.compilerOptions.paths[key];
                            delete tsconfig.compilerOptions.paths[key];
                        }
                    }

                    const sortedEntries = Object.entries(tsconfig.compilerOptions.paths).sort(
                        ([a], [b]) => a.localeCompare(b),
                    );
                    const sortedPaths = Object.fromEntries(sortedEntries);
                    Object.assign(sortedPaths, tail);
                    tsconfig.compilerOptions.paths = sortedPaths;

                    return JSON.stringify(tsconfig, null, 4) + "\n";
                },
            });

            return actions;
        },
    });

    plop.setHelper("capitalize", toCamel);
    plop.setHelper("kebab", toKebab);
    plop.setHelper("camelcase", toCamel);
    plop.setHelper("pascal", toPascal);
    plop.setWelcomeMessage("Building faster by Equal");
}
