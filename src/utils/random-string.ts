export const randomString = (length: number = 16): string => {
    // randomUUID gives 32 hex chars once the dashes are stripped
    return crypto.randomUUID().replace(/-/g, '').slice(0, length)
}
