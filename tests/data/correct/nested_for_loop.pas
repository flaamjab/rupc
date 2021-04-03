program NestedForLoop;

var
  a, b: integer;

begin
  for a := 0 to 3 do
    for b := 0 to 3 do begin
      writeln_int(a);
      writeln_int(b);
      writeln_int(0)
    end
end.
