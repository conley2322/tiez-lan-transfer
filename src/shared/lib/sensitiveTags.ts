/**
 * Tags that make an entry sensitive: stored encrypted and hidden in previews.
 * Same names as SENSITIVE_TAGS in database.rs, and like there, any case.
 */
export const SENSITIVE_TAG_NAMES = ["sensitive", "密码", "password"] as const;

export const isSensitiveTagName = (name: string): boolean => {
    const normalized = name.trim().toLowerCase();
    return SENSITIVE_TAG_NAMES.some((tag) => tag === normalized);
};

export const hasSensitiveTag = (tags?: readonly string[]): boolean =>
    Boolean(tags?.some(isSensitiveTagName));
