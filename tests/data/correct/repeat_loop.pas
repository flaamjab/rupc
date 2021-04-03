program RepeatLoop;

var
  a: integer;

begin
  a := 0;
  repeat
    a := a + 1;
    writeln_int(a)
  until a = 10;
end.
