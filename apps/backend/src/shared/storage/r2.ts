import { S3Client, PutObjectCommand, GetObjectCommand, HeadBucketCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

export type R2Bucket = "releases" | "private";

function bucketName(bucket: R2Bucket): string | undefined {
    return bucket === "releases" ? process.env.R2_BUCKET_RELEASES : process.env.R2_BUCKET_PRIVATE;
}

export function isR2Configured(): boolean {
    return Boolean(
        process.env.R2_ACCOUNT_ID &&
            process.env.R2_ACCESS_KEY_ID &&
            process.env.R2_SECRET_ACCESS_KEY &&
            process.env.R2_BUCKET_RELEASES &&
            process.env.R2_BUCKET_PRIVATE,
    );
}

let cachedClient: S3Client | null = null;

function client(): S3Client {
    if (cachedClient) return cachedClient;
    cachedClient = new S3Client({
        region: "auto",
        endpoint: `https://${process.env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com`,
        credentials: {
            accessKeyId: process.env.R2_ACCESS_KEY_ID!,
            secretAccessKey: process.env.R2_SECRET_ACCESS_KEY!,
        },
    });
    return cachedClient;
}

/** Uploads a buffer to the given bucket under `key`. Caller must check `isR2Configured()` first. */
export async function uploadObject(bucket: R2Bucket, key: string, body: Buffer, contentType?: string): Promise<void> {
    await client().send(
        new PutObjectCommand({
            Bucket: bucketName(bucket),
            Key: key,
            Body: body,
            ContentType: contentType,
        }),
    );
}

/** A short-lived signed GET URL (default 5 minutes, per docs/06-security.md's receipt/bundle downloads). */
export async function presignDownloadUrl(bucket: R2Bucket, key: string, expiresInSeconds = 300): Promise<string> {
    const command = new GetObjectCommand({ Bucket: bucketName(bucket), Key: key });
    return getSignedUrl(client(), command, { expiresIn: expiresInSeconds });
}

/** Health check for /health and admin diagnostics — never throws, returns false on any failure. */
export async function checkR2Health(): Promise<boolean> {
    if (!isR2Configured()) return true; // not configured is not "unhealthy" — it's a documented dev state
    try {
        await client().send(new HeadBucketCommand({ Bucket: bucketName("private") }));
        return true;
    } catch {
        return false;
    }
}
