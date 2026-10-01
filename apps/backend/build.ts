import "dotenv/config";
import { buildSync, type BuildOptions } from "esbuild";
import { writeFileSync } from "node:fs";

function getBaseBuildOptions(): Partial<BuildOptions> {
    return {
        entryPoints: ["./src/index.ts"],
        bundle: true,
        platform: "node",
        outdir: "dist",
        tsconfig: "tsconfig.json",
        external: [
            "pg-native",
            "argon2",
            "pino",
            "pino-http",
            "pino-pretty",
            "thread-stream",
            "prom-client",
        ],
        loader: { ".html": "text" },
        target: "node20",
        metafile: true,
    };
}

function getDevBuildOptions(): Partial<BuildOptions> {
    return {
        minify: false,
        treeShaking: false,
        sourcemap: "inline",
        logLevel: "debug",
        keepNames: true,
        legalComments: "inline",
        define: {
            "process.env.NODE_ENV": '"DEV"',
        },
    };
}

function getProdBuildOptions(): Partial<BuildOptions> {
    return {
        minify: true,
        minifySyntax: true,
        minifyWhitespace: true,
        treeShaking: true,
        pure: ["console.log", "console.debug", "console.info", "assert"],
        legalComments: "none",
        sourcemap: "external",
        logLevel: "warning",
        define: {
            NODE_ENV: '"PROD"',
            "process.env.NODE_ENV": '"PROD"',
        },
    };
}

try {
    if (process.env.NODE_ENV === "DEV") {
        process.stdout.write("\x1Bc");
    }

    const isDev = process.env.NODE_ENV === "DEV";

    const buildOptions: BuildOptions = {
        ...getBaseBuildOptions(),
        ...(isDev ? getDevBuildOptions() : getProdBuildOptions()),
    } as BuildOptions;

    const buildResult = buildSync(buildOptions);

    if (process.env.NODE_ENV === "DEV" && buildResult.metafile) {
        writeFileSync("dist/meta.json", JSON.stringify(buildResult.metafile, null, 2));
        const outputSize = buildResult.metafile.outputs["dist/index.js"]?.bytes || 0;
        console.log("Bundle analysis saved to dist/meta.json");
        console.log(`Total output size: ${(outputSize / 1024).toFixed(2)} KB`);
    }
} catch (error) {
    console.log("error", error);
    process.exit(1);
}
