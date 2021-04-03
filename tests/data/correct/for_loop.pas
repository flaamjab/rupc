program ForLoop;

var
  ix: integer;

begin
  for ix := 0 to 10 do
    writeln_int(ix);

  for ix := 10 downto 0 do begin
    writeln_int(ix)
  end;
  
end.
