# Syntax Grammar

First rule should match how to parse a complete recipe (programme)

```
program → "Recipe: " STRING NL NL recipe EOF ;
```

> NL means the "\n" newline character

## Recipe Structure

A recipe consists of declarations for ingredients (variables), followed by utensils (functions), and steps (statements)

```
recipe      → ingredients* utensils* step*
ingredients → "Ingredients" NL ingredient* NL
utensils    → "Utensils" NL utensil* NL
```

```
ingredient → ingredient → INGREDIENT_ID ( ":" expression )? ";" ;
utensil    → UTENSIL_ID function ;
```

# Steps (Statements)

Steps are the executable parts of a recipe.

```
step → "- "
expressionStep
| returnStep
| printStep
| ingredientDeclStep
```

```
functionStep →
expressionStep
| whileStep
| ifStep
| returnStep
| printStep
| ingredientDeclStep
```

```
expressionStep     → expression ";" ;
whileStep          → "stir" "(" expression ")" step ;
ifStep             → "check" "(" expression ")" "then" step ( "otherwise" step )? ;
printStep          → "taste" expression ";" ;
returnStep         → "serve" expression? ";" ;
ingredientDeclStep → "ingredient" ingredient ";" ;
block              → "{" functionStep\* "}" ;

```

# Expressions

Expressions define calculations and combinations of values.

```

expression → assignment ;

assignment → INGREDIENT_ID "is" assignment | logic_or ;

logic*or   → logic_and ( "or" logic_and )* ;
logic*and  → equality ( "and" equality )* ;
equality   → comparison ( ( "!=" | "==" ) comparison )_ ;
comparison → term ( ( ">" | ">=" | "<" | "<=" ) term )_ ;
term       → factor ( ( "-" | "+" ) factor )_ ;
factor     → unary ( ( "/" | "_" ) unary )\* ;

unary   → ( "!" | "-" ) unary | call ;
call    → primary ( "(" arguments? ")" | "." ID )\* ;
primary → "true" | "false" | "nil" | "this" | NUMBER | STRING | INGREDIENT_ID | UTENSIL_ID | "(" expression ")" ;

```

# Utility Rules

Helper rules for function declarations and calls.

```

function   → "(" parameters? ")" block ;
parameters → INGREDIENT*ID ( "," INGREDIENT_ID )* ;
arguments  → expression ( "," expression )\_ ;

```

# Lexical Grammar

The lexical grammar defines how characters are grouped into tokens.

```
NUMBER → DIGIT+ ( "." DIGIT+ )? ;
STRING → "\"" <any char except "\"">_ "\"" ;
ID     → ALPHA ( ALPHA | DIGIT )_ ;
ALPHA  → "a" ... "z" | "A" ... "Z" | "\_" ;
DIGIT  → "0" ... "9" ;
```