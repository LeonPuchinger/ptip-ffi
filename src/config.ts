type ElementParser = {
    hook: RegExp,
    parser: any,
}

type LanguageConfig = {
    name: string,
    functionParsers: ElementParser[],
}

function registerLanguage(config: LanguageConfig) {

}

// example:

registerLanguage({
    name: "javascript",
    functionParsers: [
        {
            hook: /function/,
            parser: (node) => {
                // parse the function call expression
            }
        },
        {
            hook: /\(.*\)\s*=>/,
            parser: (node) => {
                // parse the function declaration
            }
        }
    ],
})
