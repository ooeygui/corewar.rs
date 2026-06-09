export const COREWAR_PROBLEM_MATCHER = '$corewar';
export const COREWAR_PROBLEM_MATCHER_REGEXP = String.raw`^(.+):(\\d+):(\\d+):\\s+(error|warning):\\s+(.+)$`;

const PARSER_ERROR_LINE_REGEXP = String.raw`^(.+?) at line (\\d+)(?::\\s+(.+))?$`;

export function buildParserProblemTransformScript(relativeFileVariable: string): string {
  return [
    `$input | ForEach-Object {`,
    `  $line = $_.ToString()`,
    `  if ($line -match '${PARSER_ERROR_LINE_REGEXP}') {`,
    `    $summary = $matches[1]`,
    `    $lineNumber = $matches[2]`,
    `    $detail = $matches[3]`,
    `    if ([string]::IsNullOrWhiteSpace($detail)) {`,
    `      $message = $summary`,
    `    } else {`,
    `      $message = "$summary: $detail"`,
    `    }`,
    `    Write-Output "${relativeFileVariable}:$lineNumber:1: error: $message"`,
    `  } else {`,
    `    Write-Output $line`,
    `  }`,
    `}`
  ].join(' ');
}
