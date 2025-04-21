use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ASTNode
{
    name : String,
    token : Option<String>,
    children : Vec<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct Parser
{
    pub tokens : Vec<String>,
    pub pos : usize,
}

impl Parser
{
    pub fn tokenize(input : &str) -> Result<Self, String>
    {
        let mut chars = input.chars().peekable();
        let mut tokens = Vec::new();

        while let Some(&c) = chars.peek()
        {
            match c
            {
                ' ' | '\n' | '\t' => { chars.next(); }
                ';' | '(' | ')' | '[' | ']' | '?' | ':' | '+' | '-' | '*' | '/' | '%' | '^' | '~' | '#' | '@' | ',' =>
                {
                    tokens.push(chars.next().unwrap().to_string());
                }
                '=' | '!' | '<' | '>' =>
                {
                    let mut op = chars.next().unwrap().to_string();
                    if let Some(&next) = chars.peek()
                    {
                        if next == '='
                        {
                            op.push(chars.next().unwrap());
                        }
                    }
                    tokens.push(op);
                }
                '&' | '|' =>
                {
                    let mut op = chars.next().unwrap().to_string();
                    if let Some(&next) = chars.peek()
                    {
                        if next == c
                        {
                            op.push(chars.next().unwrap());
                        }
                        else
                        {
                            return Err(format!("unexpected character: `{}`", c));
                        }
                    }
                    tokens.push(op);
                }
                '0'..='9' | '.' =>
                {
                    let mut num = String::new();
                    while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    {
                        num.push(chars.next().unwrap());
                    }
                    if chars.peek() == Some(&'.')
                    {
                        num.push(chars.next().unwrap());
                        while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false)
                        {
                            num.push(chars.next().unwrap());
                        }
                    }
                    tokens.push(num);
                }
                'a'..='z' | 'A'..='Z' | '_' =>
                {
                    let mut ident = String::new();
                    while chars.peek().map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false)
                    {
                        ident.push(chars.next().unwrap());
                    }
                    tokens.push(ident);
                }
                _ =>
                {
                    return Err(format!("unexpected character: `{}`", c));
                }
            }
        }

        tokens.push("0EOF".to_string());
        Ok(Self { tokens, pos : 0 })
    }

    pub fn peek(&self) -> &str
    {
        self.tokens.get(self.pos).map(String::as_str).unwrap_or("0EOF")
    }

    pub fn advance(&mut self) -> String
    {
        let tok = self.peek().to_string();
        self.pos += 1;
        tok
    }

    pub fn match_token(&mut self, expected : &str) -> bool
    {
        if self.peek() == expected
        {
            self.advance();
            true
        }
        else
        {
            false
        }
    }

    pub fn parse(&mut self) -> Option<ASTNode>
    {
        let ret = self.parse_program();
        if self.peek() != "0EOF"
        {
            return None;
        }
        ret
    }

    pub fn parse_program(&mut self) -> Option<ASTNode>
    {
        let mut children = Vec::new();

        while let Some(binding) = self.parse_binding_or_arraydef()
        {
            children.push(binding);
        }

        if let Some(expr) = self.parse_expr()
        {
            children.push(expr);
            Some(ASTNode
            {
                name : "program".to_string(),
                token : None,
                children,
            })
        }
        else
        {
            None
        }
    }
    
    pub fn parse_binding_or_arraydef(&mut self) -> Option<ASTNode>
    {
        if let Some(x) = self.parse_arraydef()
        {
            return Some(x);
        }
        if let Some(x) = self.parse_binding()
        {
            return Some(x);
        }
        None
    }
    
    pub fn parse_arraydef(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        
        if self.match_token("array")
        {
            if let Some(name) = self.parse_name()
            {
                if self.match_token("=")
                {
                    if self.match_token("[")
                    {
                        let mut ret = ASTNode
                        {
                            name : "arraydef".to_string(),
                            token : None,
                            children : vec!(name),
                        };
                        
                        while self.peek() != "0EOF" && !self.match_token(")")
                        {
                            if let Some(expr) = self.parse_literal()
                            {
                                ret.children.push(expr);
                            }
                            if self.peek() == ","
                            {
                                self.advance();
                            }
                            else if self.peek() == "]"
                            {
                                break;
                            }
                            else
                            {
                                return None;
                            }
                        }
                        self.advance();
                        
                        if self.match_token(";")
                        {
                            return Some(ret);
                        }
                    }
                }
            }
        }
        self.pos = start;
        None
    }
    
    pub fn parse_binding(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        if let Some(name) = self.parse_name()
        {
            if self.match_token("=")
            {
                if let Some(expr) = self.parse_expr()
                {
                    if self.match_token(";")
                    {
                        return Some(ASTNode
                        {
                            name : "binding".to_string(),
                            token : None,
                            children : vec!(name, expr),
                        });
                    }
                }
            }
        }
        self.pos = start;
        None
    }
    
    pub fn parse_expr(&mut self) -> Option<ASTNode>
    {
        self.parse_binexpr5()
    }

    pub fn parse_binexpr(&mut self, lower_rule : &str, ops : Vec<&str>) -> Option<ASTNode>
    {
        let mut left = self.call_rule(lower_rule)?;
        while ops.contains(&self.peek())
        {
            let op = self.advance();
            let right = self.call_rule(lower_rule)?;
            left = ASTNode
            {
                name : "binexpr".to_string(),
                token : Some(op),
                children : vec!(left, right),
            };
        }
        Some(left)
    }

    pub fn parse_binexpr5(&mut self) -> Option<ASTNode>
    {
        self.parse_binexpr("binexpr4", vec!("&&", "||"))
    }

    pub fn parse_binexpr4(&mut self) -> Option<ASTNode>
    {
        self.parse_binexpr("binexpr3", vec!("==", "!=", ">", "<", ">=", "<="))
    }

    pub fn parse_binexpr3(&mut self) -> Option<ASTNode>
    {
        self.parse_binexpr("binexpr2", vec!("+", "-"))
    }

    pub fn parse_binexpr2(&mut self) -> Option<ASTNode>
    {
        self.parse_binexpr("binexpr1", vec!("*", "/", "%"))
    }
    
    pub fn parse_binexpr1(&mut self) -> Option<ASTNode>
    {
        let left = self.call_rule("unexpr")?;
        if self.peek() == "^"
        {
            let op = self.advance();
            let right = self.parse_binexpr1()?;
            Some(ASTNode
            {
                name: "binexpr".to_string(),
                token: Some(op),
                children: vec!(left, right),
            })
        }
        else
        {
            Some(left)
        }
    }
    
    pub fn parse_unexpr(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        let unops = vec!("+", "-", "?", "!", "~");
        if unops.contains(&self.peek())
        {
            let op = self.advance();
            if let Some(expr) = self.parse_unexpr()
            {
                return Some(ASTNode
                {
                    name : "unexpr".to_string(),
                    token : Some(op),
                    children : vec!(expr),
                });
            }
            self.pos = start;
            return None;
        }
        self.parse_val()
    }

    pub fn parse_literal(&mut self) -> Option<ASTNode>
    {
        if self.peek().chars().all(|c| c.is_ascii_digit() || c == '.')
            && self.peek() != "."
            && self.peek().chars().filter(|c| *c == '.').count() <= 1
        {
            return Some(ASTNode
            {
                name : "literal".to_string(),
                token : Some(self.advance()),
                children : vec!(),
            });
        }
        None
    }
    
    pub fn parse_val(&mut self) -> Option<ASTNode>
    {
        if let Some(t) = self.parse_ternary()
        {
            return Some(t);
        }
        
        let start = self.pos;
        if self.peek() == "("
        {
            self.advance();
            if let Some(expr) = self.parse_expr()
            {
                if self.match_token(")")
                {
                    return Some(ASTNode
                    {
                        name : "group".to_string(),
                        token : None,
                        children : vec!(expr),
                    });
                }
            }
            return None;
        }
        self.pos = start;
        
        if let Some(t) = self.parse_literal()
        {
            return Some(t);
        }
        
        if let Some(t) = self.parse_fcall()
        {
            return Some(t);
        }
        
        if let Some(t) = self.parse_arrayaccess()
        {
            return Some(t);
        }
        
        self.parse_name()
    }

    pub fn parse_arrayaccess(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        let name = self.peek();
        if name.chars().next().map(char::is_alphabetic).unwrap_or(false)
        {
            let name = self.advance();
            if self.match_token("[")
            {
                if let Some(index) = self.parse_expr()
                {
                    if self.match_token("]")
                    {
                        return Some(ASTNode
                        {
                            name : "arrayaccess".to_string(),
                            token : Some(name),
                            children : vec!(index),
                        });
                    }
                }
            }
            
        }
        self.pos = start;
        None
    }

    pub fn parse_ternary(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        if self.peek() == "("
        {
            self.advance();
            if let Some(cond) = self.parse_expr()
            {
                if self.match_token("?")
                {
                    if let Some(t_branch) = self.parse_expr()
                    {
                        if self.match_token(":")
                        {
                            if let Some(f_branch) = self.parse_expr()
                            {
                                if self.match_token(")")
                                {
                                    return Some(ASTNode
                                    {
                                        name : "ternary".to_string(),
                                        token : None,
                                        children : vec!(cond, t_branch, f_branch),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        self.pos = start;
        None
    }

    pub fn parse_fcall(&mut self) -> Option<ASTNode>
    {
        let start = self.pos;
        let name = self.peek();
        if name.chars().next().map(char::is_alphabetic).unwrap_or(false)
        {
            let name = self.advance();
            if self.peek() == "("
            {
                self.advance();
                let mut ret = ASTNode
                {
                    name : "fcall".to_string(),
                    token : Some(name),
                    children : vec!(),
                };
                
                while self.peek() != "0EOF" && !self.match_token(")")
                {
                    if let Some(expr) = self.parse_expr()
                    {
                        ret.children.push(expr);
                    }
                    if self.peek() == ","
                    {
                        self.advance();
                    }
                    else if self.peek() == ")"
                    {
                        break;
                    }
                    else
                    {
                        return None;
                    }
                }
                self.advance();
                return Some(ret);
            }
            
        }
        self.pos = start;
        None
    }

    pub fn parse_name(&mut self) -> Option<ASTNode>
    {
        let name = self.peek();
        if name.chars().next().map(char::is_alphabetic).unwrap_or(false)
        {
            return Some(ASTNode
            {
                name : "name".to_string(),
                token : Some(self.advance()),
                children : vec!(),
            });
        }
        None
    }

    pub fn call_rule(&mut self, rule : &str) -> Option<ASTNode>
    {
        match rule
        {
            "binexpr1" => self.parse_binexpr1(),
            "binexpr2" => self.parse_binexpr2(),
            "binexpr3" => self.parse_binexpr3(),
            "binexpr4" => self.parse_binexpr4(),
            "binexpr5" => self.parse_binexpr5(),
            "unexpr" => self.parse_unexpr(),
            _ => panic!("unknown rule : {}", rule),
        }
    }
}


pub struct Compiler
{
    vars : HashSet<String>,
}

impl Compiler
{
    pub fn compile_program(ast : &ASTNode, varnames : &[String]) -> Result<String, String>
    {
        let mut compiler = Self { vars : varnames.iter().cloned().collect::<HashSet<String>>() };
        let mut glsl_code = String::new();
        
        for arraydef in ast.children.iter().filter(|&child| child.name == "arraydef")
        {
            let binding_code = compiler.compile(arraydef)?;
            glsl_code.push_str(&binding_code);
            glsl_code.push_str(";\n");
        }
        
        for binding in ast.children.iter().filter(|&child| child.name == "binding")
        {
            let binding_code = compiler.compile(binding)?;
            glsl_code.push_str(&binding_code);
            glsl_code.push_str(";\n");
        }
        
        let final_expr_code = compiler.compile(ast.children.last().unwrap())?;
        glsl_code.push_str("return (");
        glsl_code.push_str(&final_expr_code);
        glsl_code.push_str(");\n");
        
        Ok(glsl_code)
    }

    pub fn compile(&mut self, expr : &ASTNode) -> Result<String, String>
    {
        let js = false;
        Ok(match expr.name.as_str()
        {
            "binexpr" =>
            {
                let left = self.compile(&expr.children[0])?;
                let op = expr.token.clone().unwrap();
                let right = self.compile(&expr.children[1])?;
                if op == "^"
                {
                    if js
                    {
                        format!("(Math.pow({}, {}))", left, right)
                    }
                    else
                    {
                        format!("(pow({}, {}))", left, right)
                    }
                }
                else if matches!(op.as_str(), "<" | ">" | "<=" | ">=" | "==" | "!=" | "&&" | "||")
                {
                    format!("(float({} {} {}))", left, op, right)
                }
                else
                {
                    format!("({} {} {})", left, op, right)
                }
            }
            "literal" =>
            {
                let mut lit = expr.token.clone().unwrap();
                if !lit.contains(".")
                {
                    lit += ".0";
                }
                format!("({})", lit)
            }
            "name" =>
            {
                let name = expr.token.clone().unwrap();
                if self.vars.contains(&name)
                {
                    format!("({})", name)
                }
                else
                {
                    "(0.0)".to_string()
                }
            }
            "arrayaccess" =>
            {
                let name = expr.token.clone().unwrap();
                let n = self.compile(&expr.children[0])?;
                format!("({}[int({})])", name, n)
            }
            "arraydef" =>
            {
                let name = expr.children[0].token.clone().unwrap();
                
                let mut ret;
                if js 
                {
                    ret = format!("let {} = [", name);
                }
                else
                {
                    ret = format!("float {}[{}] = float[](", name, expr.children.len() - 1);
                }
                for (i, child) in expr.children.iter().skip(1).enumerate()
                {
                    let n = self.compile(child)?;
                    ret += &n;
                    if i + 1 < expr.children.len() - 1
                    {
                        ret += ", ";
                    }
                }
                if js 
                {
                    ret += "]";
                }
                else
                {
                    ret += ")";
                }
                ret
            }
            "fcall" =>
            {
                let mut name = expr.token.clone().unwrap();
                let argcount = match name.as_str()
                {
                    "abs" => 1,
                    
                    "log" => 1,
                    "log2" => 1,
                    "exp" => 1,
                    "exp2" => 1,
                    "sqrt" => 1,
                    
                    "round" => 1,
                    "ceil" => 1,
                    "floor" => 1,
                    "trunc" => 1,
                    "fract" => 1,
                    
                    "sin" => 1,
                    "cos" => 1,
                    "tan" => 1,
                    "asin" => 1,
                    "acos" => 1,
                    "atan" => 1,
                    "atan2" => 2,
                    
                    _unk => { return Err(format!("unknown function name `{}`", _unk)); }
                };
                if argcount != expr.children.len()
                {
                    return Err(format!("wrong number of arguments to function with name `{}`", name));
                }
                if js && name == "fract"
                {
                    format!("((x => x - Math.floor(x))({}))", self.compile(&expr.children[0])?)
                }
                else if js && name == "exp2"
                {
                    format!("(pow(2.0, {}))", self.compile(&expr.children[0])?)
                }
                else if !js && name == "atan2"
                {
                    format!("(atan({}, {}))", self.compile(&expr.children[0])?, self.compile(&expr.children[1])?)
                }
                else
                {
                    if js { name = format!("Math.{}", name); }
                    let mut ret = "(".to_string() + &name + "(";
                    for (i, child) in expr.children.iter().enumerate()
                    {
                        let n = self.compile(child)?;
                        ret += &n;
                        if i + 1 < expr.children.len()
                        {
                            ret += ", ";
                        }
                    }
                    ret += "))";
                    ret
                }
            }
            "unexpr" =>
            {
                let op = expr.token.clone().unwrap();
                let inner = self.compile(&expr.children[0])?;
                if op == "!"
                {
                    format!("({} == 0.0)", inner)
                }
                else if op == "?"
                {
                    format!("(!({} == 0.0))", inner)
                }
                else if op == "~"
                {
                    format!("(round({}))", inner)
                }
                else
                {
                    format!("({}{})", op, inner)
                }
            }
            "group" =>
            {
                let inner = self.compile(&expr.children[0])?;
                format!("({})", inner)
            }
            "ternary" =>
            {
                let cond = self.compile(&expr.children[0])?;
                let true_branch = self.compile(&expr.children[1])?;
                let false_branch = self.compile(&expr.children[2])?;
                format!("((!({} == 0.0)) ? {} : {})", cond, true_branch, false_branch)
            }
            "binding" =>
            {
                let name = expr.children[0].token.clone().unwrap();
                let expr_code = self.compile(&expr.children[1])?;
                if self.vars.contains(&name)
                {
                    format!("{} = {}", name, expr_code)
                }
                else if js
                {
                    self.vars.insert(name.clone());
                    format!("let {} = {}", name, expr_code)
                }
                else
                {
                    self.vars.insert(name.clone());
                    format!("float {} = {}", name, expr_code)
                }
            }
            _ => panic!("Unknown expression type: {}", expr.name),
        })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    pub fn test()
    {
        let mut parser = Parser::tokenize("
            array z = [5, 153, 3];
            x = 5;
            x = z[2];
            y = 639;
            a = x + x * x + sin(x);
            b = .2;
            a^x^2
        ").unwrap();
        let parsed = parser.parse();
        println!("{:#?}", parsed);
        if parsed.is_none()
        {
            println!("got to {} (aka `{}`)", parser.pos, parser.tokens[parser.pos]);
        }
        println!("{:}", Compiler::compile_program(&parsed.unwrap(), &[]).unwrap());
    }
}