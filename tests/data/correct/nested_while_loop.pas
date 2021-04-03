program NestedWhileLoop;

var
  a, b: integer;

begin
  a := 0;

  while a < 3 do begin
    b := 0;
    writeln_int(a);
    while b < 3 do begin
      writeln_int(b);
      b := b + 1
    end;
    a := a + 1
  end
end.
